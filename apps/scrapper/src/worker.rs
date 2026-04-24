use std::time::Duration;

use anyhow::Result;
use tokio::time;
use tracing::{error, info, warn};

use crate::{config::ScrapperConfig, db::ScrapperDb, sources::SourcePoller};

pub async fn run_worker(config: ScrapperConfig) -> Result<()> {
    let db = ScrapperDb::connect(&config.database_url).await?;
    let poller = SourcePoller::new()?;
    let mut interval = time::interval(Duration::from_secs(config.poll_interval_seconds));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    info!(
        scrapper_name = %config.scrapper_name,
        openapi_base_url = %config.openapi_base_url,
        poll_interval_seconds = config.poll_interval_seconds,
        "scrapper started"
    );
    let _ = db
        .event(
            None,
            "scrapper_started",
            &format!("{} started", config.scrapper_name),
        )
        .await;

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                info!("scrapper shutdown signal received");
                let _ = db.event(None, "scrapper_stopped", &format!("{} stopped", config.scrapper_name)).await;
                break;
            }
            _ = interval.tick() => {
                if let Err(err) = run_iteration(&db, &poller).await {
                    error!(error = %err, "scrapper iteration failed");
                }
            }
        }
    }

    Ok(())
}

async fn run_iteration(db: &ScrapperDb, poller: &SourcePoller) -> Result<()> {
    let sources = db.due_sources().await?;
    info!(source_count = sources.len(), "loaded due data sources");
    for source in sources {
        match source.source_type.as_str() {
            "manual_note" | "placeholder_api" => {
                db.mark_success(&source.id).await?;
                continue;
            }
            _ => {}
        }

        match poller.poll(&source).await {
            Ok(items) => {
                let mut inserted = 0;
                for item in items {
                    if db.insert_item(&source.id, &item).await? {
                        inserted += 1;
                    }
                }
                db.mark_success(&source.id).await?;
                let _ = db
                    .event(
                        Some(&source.id),
                        "source_polled",
                        &format!("Polled {} and inserted {} new items", source.name, inserted),
                    )
                    .await;
                info!(source_id = %source.id, inserted, "source poll succeeded");
            }
            Err(err) => {
                warn!(source_id = %source.id, error = %err, "source poll failed");
                db.mark_error(&source.id, &err.to_string()).await?;
                let _ = db
                    .event(Some(&source.id), "source_poll_failed", &err.to_string())
                    .await;
            }
        }
    }
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate =
            signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
