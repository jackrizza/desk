use database::Database;
use models::{
    paper::CreatePaperOrderRequest,
    trader::{
        CreateTraderEventRequest, CreateTraderRequest, CreateTraderTradeProposalRequest,
        EngineTraderConfigResponse, Trader, TraderDetail, TraderEventsResponse, TraderRuntimeState,
        TraderTradeProposal, TraderTradeProposalsResponse, UpdateTraderRequest,
        UpsertTraderRuntimeStateRequest,
    },
};

use crate::{
    paper::{self, PaperApiError, PaperErrorKind},
    secrets::{DatabaseTraderSecretStore, TraderSecretStore},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraderErrorKind {
    BadRequest,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug)]
pub struct TraderApiError {
    pub kind: TraderErrorKind,
    pub message: String,
}

impl TraderApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: TraderErrorKind::BadRequest,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: TraderErrorKind::NotFound,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: TraderErrorKind::Conflict,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: TraderErrorKind::Internal,
            message: message.into(),
        }
    }
}

pub async fn create_trader(
    database: &Database,
    request: CreateTraderRequest,
) -> Result<Trader, TraderApiError> {
    validate_name(&request.name)?;
    validate_perspective(&request.fundamental_perspective)?;
    validate_freedom_level(&request.freedom_level)?;
    let secret = DatabaseTraderSecretStore::new(database)
        .normalize_openai_key(&request.openai_api_key)
        .map_err(TraderApiError::bad_request)?;
    let now = chrono::Utc::now().to_rfc3339();
    let trader = Trader {
        id: uuid::Uuid::now_v7().to_string(),
        name: request.name.trim().to_string(),
        fundamental_perspective: request.fundamental_perspective.trim().to_string(),
        freedom_level: request.freedom_level.trim().to_string(),
        status: "stopped".to_string(),
        default_paper_account_id: request
            .default_paper_account_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
        started_at: None,
        stopped_at: None,
    };

    database
        .create_trader(&trader, &request.info_sources, &secret)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to create trader: {err}")))
}

pub async fn list_traders(database: &Database) -> Result<Vec<Trader>, TraderApiError> {
    database
        .list_traders()
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to list traders: {err}")))
}

pub async fn get_trader_detail(
    database: &Database,
    trader_id: &str,
) -> Result<TraderDetail, TraderApiError> {
    let trader = database
        .get_trader(trader_id)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load trader: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("trader not found"))?;
    let info_sources = database
        .list_trader_info_sources(trader_id)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load info sources: {err}")))?;
    let runtime_state = database
        .get_trader_runtime_state(trader_id)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load runtime state: {err}")))?;
    let recent_events = database
        .list_trader_events(trader_id, 25)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load trader events: {err}")))?;

    Ok(TraderDetail {
        trader,
        info_sources,
        runtime_state,
        recent_events,
    })
}

pub async fn update_trader(
    database: &Database,
    trader_id: &str,
    request: UpdateTraderRequest,
) -> Result<Trader, TraderApiError> {
    if let Some(name) = &request.name {
        validate_name(name)?;
    }
    if let Some(perspective) = &request.fundamental_perspective {
        validate_perspective(perspective)?;
    }
    if let Some(freedom_level) = &request.freedom_level {
        validate_freedom_level(freedom_level)?;
    }

    let secret = match request.openai_api_key.as_deref() {
        Some(key) => Some(
            DatabaseTraderSecretStore::new(database)
                .normalize_openai_key(key)
                .map_err(TraderApiError::bad_request)?,
        ),
        None => None,
    };
    let default_account = request
        .default_paper_account_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    database
        .update_trader(
            trader_id,
            request.name.as_deref().map(str::trim),
            request.fundamental_perspective.as_deref().map(str::trim),
            request.freedom_level.as_deref().map(str::trim),
            default_account,
            secret.as_deref(),
            request.info_sources.as_deref(),
        )
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to update trader: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("trader not found"))
}

pub async fn delete_trader(database: &Database, trader_id: &str) -> Result<(), TraderApiError> {
    match database.delete_trader(trader_id).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(TraderApiError::not_found("trader not found")),
        Err(err) => Err(TraderApiError::internal(format!(
            "failed to delete trader: {err}"
        ))),
    }
}

pub async fn set_status(
    database: &Database,
    trader_id: &str,
    status: &str,
) -> Result<Trader, TraderApiError> {
    let (event_type, message) = match status {
        "running" => ("trader_started", "Trader started"),
        "stopped" => ("trader_stopped", "Trader stopped"),
        "paused" => ("trader_paused", "Trader paused"),
        _ => return Err(TraderApiError::bad_request("invalid trader status")),
    };

    database
        .set_trader_status(trader_id, status, event_type, message)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to update trader status: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("trader not found"))
}

pub async fn list_events(
    database: &Database,
    trader_id: &str,
) -> Result<TraderEventsResponse, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    Ok(TraderEventsResponse {
        events: database
            .list_trader_events(trader_id, 100)
            .await
            .map_err(|err| TraderApiError::internal(format!("failed to list events: {err}")))?,
    })
}

pub async fn get_runtime_state(
    database: &Database,
    trader_id: &str,
) -> Result<TraderRuntimeState, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    database
        .get_trader_runtime_state(trader_id)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load runtime state: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("runtime state not found"))
}

pub async fn list_trade_proposals(
    database: &Database,
    trader_id: &str,
) -> Result<TraderTradeProposalsResponse, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    Ok(TraderTradeProposalsResponse {
        proposals: database
            .list_trader_trade_proposals(trader_id)
            .await
            .map_err(|err| TraderApiError::internal(format!("failed to list proposals: {err}")))?,
    })
}

pub async fn approve_trade_proposal(
    database: &Database,
    cache: &cache::Cache,
    trader_id: &str,
    proposal_id: &str,
) -> Result<TraderTradeProposal, TraderApiError> {
    let trader = ensure_trader_exists(database, trader_id).await?;
    let proposal = database
        .get_trader_trade_proposal(trader_id, proposal_id)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load proposal: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("proposal not found"))?;
    if proposal.status != "pending_review" {
        return Err(TraderApiError::conflict("proposal is not pending review"));
    }
    let account_id = trader
        .default_paper_account_id
        .ok_or_else(|| TraderApiError::conflict("trader has no paper account selected"))?;

    let order = paper::execute_paper_market_order(
        database,
        cache,
        CreatePaperOrderRequest {
            account_id,
            symbol: proposal.symbol.clone(),
            side: proposal.side.clone(),
            order_type: proposal.order_type.clone(),
            quantity: proposal.quantity,
            requested_price: None,
            source: Some("trader".to_string()),
            trader_id: Some(trader_id.to_string()),
            strategy_id: None,
            signal_id: None,
            proposal_id: Some(proposal_id.to_string()),
        },
    )
    .await
    .map_err(map_paper_error)?;

    let updated = database
        .update_trader_trade_proposal_review(
            trader_id,
            proposal_id,
            "executed",
            Some(&order.order.id),
        )
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to update proposal: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("proposal not found"))?;
    let _ = database
        .create_trader_event(
            trader_id,
            &CreateTraderEventRequest {
                event_type: "trade_proposal_approved".to_string(),
                message: format!("Approved and executed proposal {}", proposal_id),
                payload: Some(format!(r#"{{"order_id":"{}"}}"#, order.order.id)),
            },
        )
        .await;
    Ok(updated)
}

pub async fn reject_trade_proposal(
    database: &Database,
    trader_id: &str,
    proposal_id: &str,
) -> Result<TraderTradeProposal, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    let proposal = database
        .get_trader_trade_proposal(trader_id, proposal_id)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load proposal: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("proposal not found"))?;
    if proposal.status != "pending_review" {
        return Err(TraderApiError::conflict("proposal is not pending review"));
    }
    let updated = database
        .update_trader_trade_proposal_review(trader_id, proposal_id, "rejected", None)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to reject proposal: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("proposal not found"))?;
    let _ = database
        .create_trader_event(
            trader_id,
            &CreateTraderEventRequest {
                event_type: "trade_proposal_rejected".to_string(),
                message: format!("Rejected proposal {}", proposal_id),
                payload: None,
            },
        )
        .await;
    Ok(updated)
}

pub async fn engine_config(
    database: &Database,
) -> Result<EngineTraderConfigResponse, TraderApiError> {
    database
        .list_engine_trader_configs()
        .await
        .map(|traders| EngineTraderConfigResponse { traders })
        .map_err(|err| TraderApiError::internal(format!("failed to list engine traders: {err}")))
}

pub async fn upsert_runtime_state(
    database: &Database,
    trader_id: &str,
    request: UpsertTraderRuntimeStateRequest,
) -> Result<TraderRuntimeState, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    database
        .upsert_trader_runtime_state(trader_id, &request)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to upsert runtime state: {err}")))
}

pub async fn create_event(
    database: &Database,
    trader_id: &str,
    request: CreateTraderEventRequest,
) -> Result<models::trader::TraderEvent, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    database
        .create_trader_event(trader_id, &request)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to create event: {err}")))
}

pub async fn create_trade_proposal(
    database: &Database,
    trader_id: &str,
    request: CreateTraderTradeProposalRequest,
) -> Result<TraderTradeProposal, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    validate_trade_proposal(&request)?;
    database
        .create_trader_trade_proposal(trader_id, &request)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to create proposal: {err}")))
}

async fn ensure_trader_exists(
    database: &Database,
    trader_id: &str,
) -> Result<Trader, TraderApiError> {
    database
        .get_trader(trader_id)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load trader: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("trader not found"))
}

fn validate_name(name: &str) -> Result<(), TraderApiError> {
    if name.trim().is_empty() {
        return Err(TraderApiError::bad_request("name must be non-empty"));
    }
    Ok(())
}

fn validate_perspective(perspective: &str) -> Result<(), TraderApiError> {
    if perspective.trim().is_empty() {
        return Err(TraderApiError::bad_request(
            "fundamental_perspective must be non-empty",
        ));
    }
    Ok(())
}

fn validate_freedom_level(freedom_level: &str) -> Result<(), TraderApiError> {
    match freedom_level.trim() {
        "analyst" | "junior_trader" | "senior_trader" => Ok(()),
        _ => Err(TraderApiError::bad_request("invalid freedom_level")),
    }
}

fn validate_trade_proposal(
    request: &CreateTraderTradeProposalRequest,
) -> Result<(), TraderApiError> {
    if request.symbol.trim().is_empty() {
        return Err(TraderApiError::bad_request("symbol must be non-empty"));
    }
    match request.side.trim().to_ascii_lowercase().as_str() {
        "buy" | "sell" => {}
        _ => return Err(TraderApiError::bad_request("side must be buy or sell")),
    }
    if request.quantity <= 0.0 {
        return Err(TraderApiError::bad_request(
            "quantity must be greater than 0",
        ));
    }
    if request
        .order_type
        .as_deref()
        .unwrap_or("market")
        .trim()
        .to_ascii_lowercase()
        != "market"
    {
        return Err(TraderApiError::bad_request(
            "only market orders are supported",
        ));
    }
    if request.reason.trim().is_empty() {
        return Err(TraderApiError::bad_request("reason must be non-empty"));
    }
    Ok(())
}

fn map_paper_error(error: PaperApiError) -> TraderApiError {
    match error.kind {
        PaperErrorKind::BadRequest => TraderApiError::bad_request(error.message),
        PaperErrorKind::NotFound => TraderApiError::not_found(error.message),
        PaperErrorKind::Conflict => TraderApiError::conflict(error.message),
        PaperErrorKind::Internal => TraderApiError::internal(error.message),
    }
}
