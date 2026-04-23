use std::{sync::Arc, time::Duration};

use anyhow::Result;
use cache::Cache;
use chrono::Utc;
use tokio::time;
use tracing::{error, info, warn};

use crate::{
    client::OpenApiClient,
    config::EngineConfig,
    strategy_runner::run_strategy,
    types::{EngineEventRequest, EngineHeartbeatRequest},
};

const DEFAULT_RANGE: &str = "1d";
const DEFAULT_INTERVAL: &str = "1m";
const DEFAULT_PREPOST: bool = false;

pub async fn run_engine(config: EngineConfig) -> Result<()> {
    let client = OpenApiClient::new(config.openapi_base_url.clone());
    let cache = Arc::new(Cache::new("cache_data".to_string()));
    let mut poll_interval = time::interval(Duration::from_secs(config.poll_interval_seconds));
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    poll_interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    info!(
        engine_name = %config.engine_name,
        openapi_base_url = %config.openapi_base_url,
        poll_interval_seconds = config.poll_interval_seconds,
        enable_test_paper_orders = config.enable_test_paper_orders,
        "engine startup"
    );

    send_heartbeat(&client, &config.engine_name, "starting").await;

    match client.health_check().await {
        Ok(response) => info!(status = %response.status, "openapi health check succeeded"),
        Err(err) => warn!(error = %err, "openapi health check failed during startup"),
    }

    send_heartbeat(&client, &config.engine_name, "running").await;
    poll_interval.tick().await;

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                info!("shutdown signal received");
                send_heartbeat(&client, &config.engine_name, "stopping").await;
                break;
            }
            _ = poll_interval.tick() => {
                if let Err(err) = run_iteration(&config, &client, cache.clone()).await {
                    error!(error = %err, "engine loop iteration failed");
                }
            }
        }
    }

    info!("engine shutdown complete");
    Ok(())
}

async fn run_iteration(
    config: &EngineConfig,
    client: &OpenApiClient,
    cache: Arc<Cache>,
) -> Result<()> {
    info!("starting engine poll iteration");

    let _active_symbols = match client.fetch_active_symbols().await {
        Ok(symbols) => {
            info!(symbols = ?symbols, "received active symbols from openapi");
            symbols
        }
        Err(err) => {
            warn!(error = %err, "failed to fetch active symbols");
            report_event(
                client,
                &config.engine_name,
                "active_symbols_fetch_failed",
                None,
                format!("failed to fetch active symbols: {err}"),
            )
            .await;
            return Ok(());
        }
    };

    let strategy_configs = match client.fetch_engine_strategy_configs().await {
        Ok(configs) => {
            info!(
                strategy_count = configs.len(),
                "received engine strategy configs from openapi"
            );
            configs
        }
        Err(err) => {
            warn!(error = %err, "failed to fetch strategy configs");
            report_event(
                client,
                &config.engine_name,
                "strategy_config_fetch_failed",
                None,
                format!("failed to fetch strategy configs: {err}"),
            )
            .await;
            return Ok(());
        }
    };

    for strategy in strategy_configs {
        let symbols = strategy.symbol_universe.clone();

        if symbols.is_empty() {
            report_event(
                client,
                &config.engine_name,
                "strategy_evaluated",
                None,
                format!(
                    "strategy {} evaluated with no symbols configured",
                    strategy.strategy_id
                ),
            )
            .await;
            continue;
        }

        for symbol in &symbols {
            info!(
                strategy_id = %strategy.strategy_id,
                account_id = %strategy.paper_account_id,
                symbol = %symbol,
                "evaluating strategy symbol"
            );

            let cache_key =
                format!("{symbol}_{DEFAULT_RANGE}_{DEFAULT_INTERVAL}_{DEFAULT_PREPOST}");

            match cache.check_cache(&cache_key).await {
                Ok(data) => {
                    let entry_count = data.stock_data.len();
                    let closes = data
                        .stock_data
                        .iter()
                        .filter_map(|entry| entry.close.parse::<f64>().ok())
                        .collect::<Vec<_>>();
                    info!(
                        strategy_id = %strategy.strategy_id,
                        symbol = %symbol,
                        entries = entry_count,
                        "market data refreshed for strategy symbol"
                    );
                    report_event(
                        client,
                        &config.engine_name,
                        "strategy_evaluated",
                        Some(symbol.clone()),
                        format!(
                            "strategy {} evaluated on {} with {} cached data points",
                            strategy.strategy_id, symbol, entry_count
                        ),
                    )
                    .await;
                    if let Err(err) = run_strategy(config, client, &strategy, symbol, closes).await
                    {
                        warn!(
                            strategy_id = %strategy.strategy_id,
                            symbol = %symbol,
                            error = %err,
                            "strategy runner failed"
                        );
                        report_event(
                            client,
                            &config.engine_name,
                            "strategy_runner_failed",
                            Some(symbol.clone()),
                            format!(
                                "strategy {} failed on {}: {err}",
                                strategy.strategy_id, symbol
                            ),
                        )
                        .await;
                    }
                }
                Err(err) => {
                    warn!(
                        strategy_id = %strategy.strategy_id,
                        symbol = %symbol,
                        error = %err,
                        "failed to evaluate strategy symbol"
                    );
                    report_event(
                        client,
                        &config.engine_name,
                        "symbol_evaluation_failed",
                        Some(symbol.clone()),
                        format!(
                            "failed to evaluate strategy {} symbol {}: {err}",
                            strategy.strategy_id, symbol
                        ),
                    )
                    .await;
                }
            }
        }
    }

    Ok(())
}

async fn send_heartbeat(client: &OpenApiClient, engine_name: &str, status: &str) {
    let request = EngineHeartbeatRequest {
        engine_name: engine_name.to_string(),
        status: status.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    match client.send_heartbeat(&request).await {
        Ok(()) => info!(status, "heartbeat sent successfully"),
        Err(err) => warn!(status, error = %err, "heartbeat send failed"),
    }
}

async fn report_event(
    client: &OpenApiClient,
    engine_name: &str,
    event_type: &str,
    symbol: Option<String>,
    message: String,
) {
    let request = EngineEventRequest {
        engine_name: engine_name.to_string(),
        event_type: event_type.to_string(),
        symbol,
        message,
        timestamp: Utc::now().to_rfc3339(),
    };

    match client.report_engine_event(&request).await {
        Ok(()) => info!(event_type, "engine event reported successfully"),
        Err(err) => warn!(event_type, error = %err, "engine event reporting failed"),
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate =
            signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");

        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(err) = result {
                    warn!(error = %err, "failed to listen for Ctrl+C shutdown signal");
                }
            }
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        if let Err(err) = tokio::signal::ctrl_c().await {
            warn!(error = %err, "failed to listen for Ctrl+C shutdown signal");
        }
    }
}
