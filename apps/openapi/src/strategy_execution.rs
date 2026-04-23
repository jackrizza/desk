use database::Database;
use models::trading::{
    CreateStrategySignalRequest, StrategyRuntimeStateListResponse, StrategySignalListResponse,
    UpdateStrategySignalStatusRequest, UpsertStrategyRuntimeStateRequest,
};

pub async fn create_signal(
    database: &Database,
    request: CreateStrategySignalRequest,
) -> Result<models::trading::StrategySignal, String> {
    validate_signal_request(&request)?;

    database
        .create_strategy_signal(&request)
        .await
        .map_err(|err| format!("failed to create strategy signal: {err}"))
}

pub async fn upsert_runtime_state(
    database: &Database,
    request: UpsertStrategyRuntimeStateRequest,
) -> Result<models::trading::StrategyRuntimeState, String> {
    validate_runtime_state_request(&request)?;

    database
        .upsert_strategy_runtime_state(&request)
        .await
        .map_err(|err| format!("failed to upsert strategy runtime state: {err}"))
}

pub async fn update_signal(
    database: &Database,
    signal_id: &str,
    request: UpdateStrategySignalStatusRequest,
) -> Result<(), String> {
    if signal_id.trim().is_empty() {
        return Err("signal_id must be non-empty".to_string());
    }
    validate_signal_status(&request.status)?;

    database
        .update_strategy_signal(
            signal_id,
            request.order_id.as_deref(),
            &request.status,
            request.risk_decision.as_deref(),
            request.risk_reason.as_deref(),
        )
        .await
        .map_err(|err| format!("failed to update strategy signal: {err}"))
}

pub async fn get_runtime_state(
    database: &Database,
    strategy_id: &str,
) -> Result<StrategyRuntimeStateListResponse, String> {
    ensure_strategy_exists(database, strategy_id).await?;

    let states = database
        .list_strategy_runtime_state(strategy_id)
        .await
        .map_err(|err| format!("failed to load strategy runtime state: {err}"))?;

    Ok(StrategyRuntimeStateListResponse { states })
}

pub async fn get_signals(
    database: &Database,
    strategy_id: &str,
) -> Result<StrategySignalListResponse, String> {
    ensure_strategy_exists(database, strategy_id).await?;

    let signals = database
        .list_strategy_signals(strategy_id)
        .await
        .map_err(|err| format!("failed to load strategy signals: {err}"))?;

    Ok(StrategySignalListResponse { signals })
}

async fn ensure_strategy_exists(database: &Database, strategy_id: &str) -> Result<(), String> {
    let project = database
        .get_project(strategy_id)
        .await
        .map_err(|err| format!("failed to load strategy: {err}"))?;

    if project.is_none() {
        return Err("strategy not found".to_string());
    }

    Ok(())
}

fn validate_signal_request(request: &CreateStrategySignalRequest) -> Result<(), String> {
    if request.strategy_id.trim().is_empty() {
        return Err("strategy_id must be non-empty".to_string());
    }
    if request.paper_account_id.trim().is_empty() {
        return Err("paper_account_id must be non-empty".to_string());
    }
    if request.symbol.trim().is_empty() {
        return Err("symbol must be non-empty".to_string());
    }
    if request.reason.trim().is_empty() {
        return Err("reason must be non-empty".to_string());
    }

    match request.signal_type.as_str() {
        "enter_long" | "exit_long" | "hold" | "no_action" => Ok(()),
        _ => Err("invalid signal_type".to_string()),
    }
    .and_then(|_| validate_signal_status(request.status.as_deref().unwrap_or("created")))
}

fn validate_runtime_state_request(
    request: &UpsertStrategyRuntimeStateRequest,
) -> Result<(), String> {
    if request.strategy_id.trim().is_empty() {
        return Err("strategy_id must be non-empty".to_string());
    }
    if request.paper_account_id.trim().is_empty() {
        return Err("paper_account_id must be non-empty".to_string());
    }
    if request.symbol.trim().is_empty() {
        return Err("symbol must be non-empty".to_string());
    }
    if request.position_state.trim().is_empty() {
        return Err("position_state must be non-empty".to_string());
    }

    Ok(())
}

fn validate_signal_status(status: &str) -> Result<(), String> {
    match status {
        "created" | "submitted" | "filled" | "blocked_by_risk" | "failed" => Ok(()),
        _ => Err("invalid signal status".to_string()),
    }
}
