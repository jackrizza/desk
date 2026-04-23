use database::Database;
use models::trading::{StrategyRiskConfig, StrategyTradingConfig, UpdateStrategyRiskConfigRequest};
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyRiskErrorKind {
    BadRequest,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug)]
pub struct StrategyRiskError {
    pub kind: StrategyRiskErrorKind,
    pub message: String,
}

impl StrategyRiskError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: StrategyRiskErrorKind::BadRequest,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: StrategyRiskErrorKind::NotFound,
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: StrategyRiskErrorKind::Conflict,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: StrategyRiskErrorKind::Internal,
            message: message.into(),
        }
    }
}

pub async fn get_risk_config(
    database: &Database,
    strategy_id: &str,
) -> Result<StrategyRiskConfig, StrategyRiskError> {
    ensure_strategy_exists(database, strategy_id).await?;

    let config = database
        .get_strategy_risk_config(strategy_id)
        .await
        .map_err(|err| StrategyRiskError::internal(format!("failed to load risk config: {err}")))?;

    Ok(config.unwrap_or_else(|| Database::default_strategy_risk_config(strategy_id)))
}

pub async fn update_risk_config(
    database: &Database,
    strategy_id: &str,
    request: UpdateStrategyRiskConfigRequest,
) -> Result<StrategyRiskConfig, StrategyRiskError> {
    ensure_strategy_exists(database, strategy_id).await?;
    let normalized = validate_update_request(database, strategy_id, request).await?;

    info!(
        strategy_id = %strategy_id,
        is_trading_enabled = normalized.is_trading_enabled,
        kill_switch_enabled = normalized.kill_switch_enabled,
        "updating strategy risk config"
    );

    database
        .upsert_strategy_risk_config(&normalized)
        .await
        .map_err(|err| StrategyRiskError::internal(format!("failed to save risk config: {err}")))
}

pub async fn trigger_kill_switch(
    database: &Database,
    strategy_id: &str,
) -> Result<StrategyRiskConfig, StrategyRiskError> {
    let mut config = get_risk_config(database, strategy_id).await?;
    config.is_trading_enabled = false;
    config.kill_switch_enabled = true;
    config.updated_at = Some(chrono::Utc::now().to_rfc3339());

    database
        .upsert_strategy_risk_config(&config)
        .await
        .map_err(|err| {
            StrategyRiskError::internal(format!("failed to activate kill switch: {err}"))
        })
}

pub async fn resume_trading(
    database: &Database,
    strategy_id: &str,
) -> Result<StrategyRiskConfig, StrategyRiskError> {
    let trading_config = database
        .get_strategy_trading_config(strategy_id)
        .await
        .map_err(|err| {
            StrategyRiskError::internal(format!("failed to load trading config: {err}"))
        })?;

    if let Some(config) = trading_config {
        if config.trading_mode == "real" {
            return Err(StrategyRiskError::conflict(
                "real trading is disabled in v1",
            ));
        }
    }

    let mut config = get_risk_config(database, strategy_id).await?;
    config.is_trading_enabled = true;
    config.kill_switch_enabled = false;
    config.updated_at = Some(chrono::Utc::now().to_rfc3339());

    database
        .upsert_strategy_risk_config(&config)
        .await
        .map_err(|err| StrategyRiskError::internal(format!("failed to resume trading: {err}")))
}

pub async fn validate_engine_order(
    database: &Database,
    strategy_id: &str,
    account_id: &str,
    symbol: &str,
    quantity: f64,
    fill_price: f64,
) -> Result<(), StrategyRiskError> {
    ensure_strategy_exists(database, strategy_id).await?;

    let trading_config = database
        .get_strategy_trading_config(strategy_id)
        .await
        .map_err(|err| {
            StrategyRiskError::internal(format!("failed to load trading config: {err}"))
        })?
        .ok_or_else(|| StrategyRiskError::conflict("strategy trading config not found"))?;

    validate_engine_trading_config(&trading_config, account_id)?;

    let risk_config = database
        .get_strategy_risk_config(strategy_id)
        .await
        .map_err(|err| StrategyRiskError::internal(format!("failed to load risk config: {err}")))?
        .unwrap_or_else(|| Database::default_strategy_risk_config(strategy_id));

    if risk_config.kill_switch_enabled {
        return Err(StrategyRiskError::conflict(
            "strategy kill switch is enabled",
        ));
    }

    if !risk_config.is_trading_enabled {
        return Err(StrategyRiskError::conflict(
            "strategy trading is disabled by risk config",
        ));
    }

    if let Some(symbols) = &risk_config.allowlist_symbols {
        if !symbols.iter().any(|allowed| allowed == symbol) {
            return Err(StrategyRiskError::conflict(
                "symbol is not allowed by risk config",
            ));
        }
    }

    if let Some(symbols) = &risk_config.blocklist_symbols {
        if symbols.iter().any(|blocked| blocked == symbol) {
            return Err(StrategyRiskError::conflict(
                "symbol is blocked by risk config",
            ));
        }
    }

    if let Some(limit) = risk_config.max_quantity_per_trade {
        if quantity > limit {
            return Err(StrategyRiskError::conflict(
                "order exceeds max quantity per trade",
            ));
        }
    }

    if let Some(limit) = risk_config.max_dollars_per_trade {
        if quantity * fill_price > limit {
            return Err(StrategyRiskError::conflict(
                "order exceeds max dollars per trade",
            ));
        }
    }

    Ok(())
}

pub async fn ensure_default_risk_config(
    database: &Database,
    strategy_id: &str,
) -> Result<StrategyRiskConfig, StrategyRiskError> {
    if let Some(existing) = database
        .get_strategy_risk_config(strategy_id)
        .await
        .map_err(|err| StrategyRiskError::internal(format!("failed to load risk config: {err}")))?
    {
        return Ok(existing);
    }

    let config = Database::default_strategy_risk_config(strategy_id);
    database
        .upsert_strategy_risk_config(&config)
        .await
        .map_err(|err| {
            StrategyRiskError::internal(format!("failed to create default risk config: {err}"))
        })
}

async fn ensure_strategy_exists(
    database: &Database,
    strategy_id: &str,
) -> Result<(), StrategyRiskError> {
    if strategy_id.trim().is_empty() {
        return Err(StrategyRiskError::bad_request(
            "strategy_id must be non-empty",
        ));
    }

    let project = database
        .get_project(strategy_id)
        .await
        .map_err(|err| StrategyRiskError::internal(format!("failed to load strategy: {err}")))?;

    if project.is_none() {
        return Err(StrategyRiskError::not_found("strategy not found"));
    }

    Ok(())
}

async fn validate_update_request(
    database: &Database,
    strategy_id: &str,
    request: UpdateStrategyRiskConfigRequest,
) -> Result<StrategyRiskConfig, StrategyRiskError> {
    let mut config = database
        .get_strategy_risk_config(strategy_id)
        .await
        .map_err(|err| StrategyRiskError::internal(format!("failed to load risk config: {err}")))?
        .unwrap_or_else(|| Database::default_strategy_risk_config(strategy_id));

    validate_positive(request.max_dollars_per_trade, "max_dollars_per_trade")?;
    validate_positive(request.max_quantity_per_trade, "max_quantity_per_trade")?;
    validate_positive(
        request.max_position_value_per_symbol,
        "max_position_value_per_symbol",
    )?;
    validate_positive(request.max_total_exposure, "max_total_exposure")?;
    validate_positive(request.max_daily_loss, "max_daily_loss")?;
    validate_non_negative_i64(request.max_open_positions, "max_open_positions")?;
    validate_non_negative_i64(request.max_daily_trades, "max_daily_trades")?;

    if request.cooldown_seconds < 0 {
        return Err(StrategyRiskError::bad_request(
            "cooldown_seconds must be >= 0",
        ));
    }

    let allowlist_symbols = normalize_symbols(request.allowlist_symbols);
    let blocklist_symbols = normalize_symbols(request.blocklist_symbols);

    if let (Some(allowlist), Some(blocklist)) = (&allowlist_symbols, &blocklist_symbols) {
        if allowlist.iter().any(|symbol| blocklist.contains(symbol)) {
            return Err(StrategyRiskError::bad_request(
                "a symbol cannot appear in both allowlist and blocklist",
            ));
        }
    }

    config.max_dollars_per_trade = request.max_dollars_per_trade;
    config.max_quantity_per_trade = request.max_quantity_per_trade;
    config.max_position_value_per_symbol = request.max_position_value_per_symbol;
    config.max_total_exposure = request.max_total_exposure;
    config.max_open_positions = request.max_open_positions;
    config.max_daily_trades = request.max_daily_trades;
    config.max_daily_loss = request.max_daily_loss;
    config.cooldown_seconds = request.cooldown_seconds;
    config.allowlist_symbols = allowlist_symbols;
    config.blocklist_symbols = blocklist_symbols;
    config.is_trading_enabled = request.is_trading_enabled;
    config.kill_switch_enabled = request.kill_switch_enabled;
    config.updated_at = Some(chrono::Utc::now().to_rfc3339());

    Ok(config)
}

fn validate_engine_trading_config(
    trading_config: &StrategyTradingConfig,
    account_id: &str,
) -> Result<(), StrategyRiskError> {
    if trading_config.trading_mode == "real" {
        return Err(StrategyRiskError::conflict(
            "real trading is disabled in v1",
        ));
    }

    if trading_config.trading_mode != "paper" || !trading_config.is_enabled {
        return Err(StrategyRiskError::conflict(
            "strategy is not enabled for paper trading",
        ));
    }

    match &trading_config.paper_account_id {
        Some(expected_account_id) if expected_account_id == account_id => Ok(()),
        _ => Err(StrategyRiskError::conflict(
            "paper account does not match persisted strategy trading config",
        )),
    }
}

fn validate_positive(value: Option<f64>, field_name: &str) -> Result<(), StrategyRiskError> {
    if let Some(value) = value {
        if value <= 0.0 {
            return Err(StrategyRiskError::bad_request(format!(
                "{field_name} must be positive",
            )));
        }
    }

    Ok(())
}

fn validate_non_negative_i64(
    value: Option<i64>,
    field_name: &str,
) -> Result<(), StrategyRiskError> {
    if let Some(value) = value {
        if value < 0 {
            return Err(StrategyRiskError::bad_request(format!(
                "{field_name} must be >= 0",
            )));
        }
    }

    Ok(())
}

fn normalize_symbols(symbols: Option<Vec<String>>) -> Option<Vec<String>> {
    symbols.and_then(|values| {
        let normalized = values
            .into_iter()
            .map(|symbol| symbol.trim().to_ascii_uppercase())
            .filter(|symbol| !symbol.is_empty())
            .collect::<Vec<_>>();

        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}
