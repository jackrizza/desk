use database::Database;
use models::trading::{
    EngineStrategyConfigResponse, StrategyTradingConfig, UpdateStrategyTradingConfigRequest,
};
use tracing::info;

use crate::strategy_risk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyTradingErrorKind {
    BadRequest,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug)]
pub struct StrategyTradingError {
    pub kind: StrategyTradingErrorKind,
    pub message: String,
}

impl StrategyTradingError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: StrategyTradingErrorKind::BadRequest,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: StrategyTradingErrorKind::NotFound,
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: StrategyTradingErrorKind::Conflict,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: StrategyTradingErrorKind::Internal,
            message: message.into(),
        }
    }
}

pub async fn get_trading_config(
    database: &Database,
    strategy_id: &str,
) -> Result<StrategyTradingConfig, StrategyTradingError> {
    ensure_strategy_exists(database, strategy_id).await?;

    let config = database
        .get_strategy_trading_config(strategy_id)
        .await
        .map_err(|err| {
            StrategyTradingError::internal(format!("failed to load trading config: {err}"))
        })?;

    Ok(config.unwrap_or_else(|| default_config(strategy_id)))
}

pub async fn update_trading_config(
    database: &Database,
    strategy_id: &str,
    request: UpdateStrategyTradingConfigRequest,
) -> Result<StrategyTradingConfig, StrategyTradingError> {
    ensure_strategy_exists(database, strategy_id).await?;
    let normalized = validate_update_request(database, strategy_id, request).await?;

    info!(
        strategy_id = %strategy_id,
        trading_mode = %normalized.trading_mode,
        paper_account_id = ?normalized.paper_account_id,
        is_enabled = normalized.is_enabled,
        "updating strategy trading config"
    );

    let saved = database
        .upsert_strategy_trading_config(&normalized)
        .await
        .map_err(|err| {
            StrategyTradingError::internal(format!("failed to save trading config: {err}"))
        })?;

    if saved.trading_mode == "paper" {
        let _ = strategy_risk::ensure_default_risk_config(database, strategy_id).await;
    }

    Ok(saved)
}

pub async fn list_engine_strategy_configs(
    database: &Database,
) -> Result<EngineStrategyConfigResponse, StrategyTradingError> {
    let strategies = database
        .list_engine_strategy_configs()
        .await
        .map_err(|err| {
            StrategyTradingError::internal(format!("failed to load engine strategy configs: {err}"))
        })?;

    Ok(EngineStrategyConfigResponse { strategies })
}

async fn ensure_strategy_exists(
    database: &Database,
    strategy_id: &str,
) -> Result<(), StrategyTradingError> {
    if strategy_id.trim().is_empty() {
        return Err(StrategyTradingError::bad_request(
            "strategy_id must be non-empty",
        ));
    }

    let project = database
        .get_project(strategy_id)
        .await
        .map_err(|err| StrategyTradingError::internal(format!("failed to load strategy: {err}")))?;

    if project.is_none() {
        return Err(StrategyTradingError::not_found("strategy not found"));
    }

    Ok(())
}

async fn validate_update_request(
    database: &Database,
    strategy_id: &str,
    request: UpdateStrategyTradingConfigRequest,
) -> Result<StrategyTradingConfig, StrategyTradingError> {
    let trading_mode = request.trading_mode.trim().to_ascii_lowercase();
    if trading_mode != "off" && trading_mode != "paper" && trading_mode != "real" {
        return Err(StrategyTradingError::bad_request(
            "invalid trading mode: must be off, paper, or real",
        ));
    }

    if trading_mode == "real" {
        return Err(StrategyTradingError::conflict(
            "real trading is disabled in v1",
        ));
    }

    let existing = database
        .get_strategy_trading_config(strategy_id)
        .await
        .map_err(|err| {
            StrategyTradingError::internal(format!("failed to load existing trading config: {err}"))
        })?;

    let mut config = existing.unwrap_or_else(|| default_config(strategy_id));
    let now = chrono::Utc::now().to_rfc3339();
    config.updated_at = Some(now.clone());

    match trading_mode.as_str() {
        "off" => {
            config.trading_mode = "off".to_string();
            config.paper_account_id = None;
            config.is_enabled = false;
            config.last_stopped_at = Some(now);
        }
        "paper" => {
            let Some(account_id) = request
                .paper_account_id
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
            else {
                return Err(StrategyTradingError::bad_request(
                    "paper mode requires paper_account_id",
                ));
            };

            let account = database
                .get_paper_account(&account_id)
                .await
                .map_err(|err| {
                    StrategyTradingError::internal(format!("failed to load paper account: {err}"))
                })?
                .ok_or_else(|| StrategyTradingError::bad_request("unknown paper_account_id"))?;

            if !account.is_active {
                return Err(StrategyTradingError::conflict("paper account is inactive"));
            }

            config.trading_mode = "paper".to_string();
            config.paper_account_id = Some(account_id);
            config.is_enabled = request.is_enabled;
            if config.is_enabled {
                config.last_started_at = Some(now);
            } else {
                config.last_stopped_at = Some(now);
            }
        }
        _ => unreachable!(),
    }

    Ok(config)
}

fn default_config(strategy_id: &str) -> StrategyTradingConfig {
    StrategyTradingConfig {
        strategy_id: strategy_id.to_string(),
        trading_mode: "off".to_string(),
        paper_account_id: None,
        is_enabled: false,
        last_started_at: None,
        last_stopped_at: None,
        created_at: None,
        updated_at: None,
    }
}
