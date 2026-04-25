use database::Database;
use models::{
    paper::CreatePaperOrderRequest,
    trader::{
        BulkUpsertTraderSymbolsRequest, CreateTraderEventRequest,
        CreateTraderPortfolioProposalActionRequest, CreateTraderPortfolioProposalRequest,
        CreateTraderRequest, CreateTraderSymbolRequest, CreateTraderTradeProposalRequest,
        EngineTraderConfigResponse, ReviewTraderPortfolioProposalRequest,
        SuggestTraderSymbolsRequest, SuggestTraderSymbolsResponse, Trader, TraderDetail,
        TraderEventsResponse, TraderPortfolioProposalDetail, TraderPortfolioProposalsResponse,
        TraderRuntimeState, TraderSymbol, TraderSymbolsResponse, TraderTradeProposal,
        TraderTradeProposalsResponse, UpdateTraderRequest, UpdateTraderSymbolRequest,
        UpsertTraderRuntimeStateRequest,
    },
};
use serde_json::{Value, json};
use std::{env, time::Duration};

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
        persona: Some(format!(
            "{} is a trader whose fundamental perspective is: {}",
            request.name.trim(),
            request.fundamental_perspective.trim()
        )),
        tone: Some("professional, concise, analytical".to_string()),
        communication_style: Some("explains uncertainty and asks for help when needed".to_string()),
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
    let tracked_symbols = database
        .list_trader_symbols(trader_id, None, None, None)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load trader symbols: {err}")))?;

    Ok(TraderDetail {
        trader,
        info_sources,
        runtime_state,
        recent_events,
        tracked_symbols,
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

pub async fn list_portfolio_proposals(
    database: &Database,
    trader_id: &str,
) -> Result<TraderPortfolioProposalsResponse, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    Ok(TraderPortfolioProposalsResponse {
        proposals: database
            .list_trader_portfolio_proposals(trader_id)
            .await
            .map_err(|err| {
                TraderApiError::internal(format!("failed to list portfolio proposals: {err}"))
            })?,
    })
}

pub async fn latest_portfolio_proposal(
    database: &Database,
    trader_id: &str,
) -> Result<TraderPortfolioProposalDetail, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    database
        .get_latest_trader_portfolio_proposal(trader_id)
        .await
        .map_err(|err| {
            TraderApiError::internal(format!("failed to load latest portfolio proposal: {err}"))
        })?
        .ok_or_else(|| TraderApiError::not_found("proposal not found"))
}

pub async fn active_portfolio_proposal(
    database: &Database,
    trader_id: &str,
) -> Result<TraderPortfolioProposalDetail, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    database
        .get_active_trader_portfolio_proposal(trader_id)
        .await
        .map_err(|err| {
            TraderApiError::internal(format!("failed to load active portfolio proposal: {err}"))
        })?
        .ok_or_else(|| TraderApiError::not_found("active proposal not found"))
}

pub async fn get_portfolio_proposal(
    database: &Database,
    trader_id: &str,
    proposal_id: &str,
) -> Result<TraderPortfolioProposalDetail, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    database
        .get_trader_portfolio_proposal(trader_id, proposal_id)
        .await
        .map_err(|err| {
            TraderApiError::internal(format!("failed to load portfolio proposal: {err}"))
        })?
        .ok_or_else(|| TraderApiError::not_found("proposal not found"))
}

pub async fn create_portfolio_proposal(
    database: &Database,
    trader_id: &str,
    mut request: CreateTraderPortfolioProposalRequest,
) -> Result<TraderPortfolioProposalDetail, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    request.trader_id = trader_id.to_string();
    validate_portfolio_proposal(&request)?;
    database
        .create_trader_portfolio_proposal(trader_id, &request)
        .await
        .map_err(|err| {
            TraderApiError::internal(format!("failed to create portfolio proposal: {err}"))
        })
}

pub async fn review_portfolio_proposal(
    database: &Database,
    trader_id: &str,
    proposal_id: &str,
    request: ReviewTraderPortfolioProposalRequest,
) -> Result<TraderPortfolioProposalDetail, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    validate_portfolio_proposal_status(&request.status)?;
    database
        .review_trader_portfolio_proposal(
            trader_id,
            proposal_id,
            request.status.trim(),
            request.review_note.as_deref().map(str::trim),
        )
        .await
        .map_err(|err| {
            TraderApiError::internal(format!("failed to review portfolio proposal: {err}"))
        })?
        .ok_or_else(|| TraderApiError::not_found("proposal not found"))
}

pub async fn accept_portfolio_proposal(
    database: &Database,
    trader_id: &str,
    proposal_id: &str,
    review_note: Option<String>,
) -> Result<TraderPortfolioProposalDetail, TraderApiError> {
    review_portfolio_proposal(
        database,
        trader_id,
        proposal_id,
        ReviewTraderPortfolioProposalRequest {
            status: "accepted".to_string(),
            review_note,
        },
    )
    .await
}

pub async fn reject_portfolio_proposal(
    database: &Database,
    trader_id: &str,
    proposal_id: &str,
    review_note: Option<String>,
) -> Result<TraderPortfolioProposalDetail, TraderApiError> {
    review_portfolio_proposal(
        database,
        trader_id,
        proposal_id,
        ReviewTraderPortfolioProposalRequest {
            status: "rejected".to_string(),
            review_note,
        },
    )
    .await
}

pub async fn list_symbols(
    database: &Database,
    trader_id: &str,
    status: Option<&str>,
    asset_type: Option<&str>,
    source: Option<&str>,
) -> Result<TraderSymbolsResponse, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    if let Some(value) = status {
        validate_symbol_status(value)?;
    }
    if let Some(value) = asset_type {
        validate_asset_type(value)?;
    }
    if let Some(value) = source {
        validate_symbol_source(value)?;
    }
    Ok(TraderSymbolsResponse {
        symbols: database
            .list_trader_symbols(trader_id, status, asset_type, source)
            .await
            .map_err(|err| TraderApiError::internal(format!("failed to list symbols: {err}")))?,
    })
}

pub async fn create_symbol(
    database: &Database,
    trader_id: &str,
    request: CreateTraderSymbolRequest,
) -> Result<TraderSymbol, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    validate_create_symbol(&request)?;
    database
        .upsert_trader_symbol(trader_id, &normalize_create_symbol(request))
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to save symbol: {err}")))
}

pub async fn update_symbol(
    database: &Database,
    trader_id: &str,
    symbol_id: &str,
    request: UpdateTraderSymbolRequest,
) -> Result<TraderSymbol, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    validate_update_symbol(&request)?;
    database
        .update_trader_symbol(trader_id, symbol_id, &request)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to update symbol: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("symbol not found"))
}

pub async fn set_symbol_status(
    database: &Database,
    trader_id: &str,
    symbol_id: &str,
    status: &str,
) -> Result<TraderSymbol, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    validate_symbol_status(status)?;
    database
        .set_trader_symbol_status(trader_id, symbol_id, status)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to update symbol status: {err}")))?
        .ok_or_else(|| TraderApiError::not_found("symbol not found"))
}

pub async fn bulk_upsert_symbols(
    database: &Database,
    trader_id: &str,
    request: BulkUpsertTraderSymbolsRequest,
) -> Result<TraderSymbolsResponse, TraderApiError> {
    ensure_trader_exists(database, trader_id).await?;
    let mut symbols = Vec::new();
    for symbol in request.symbols {
        validate_create_symbol(&symbol)?;
        symbols.push(
            database
                .upsert_trader_symbol(trader_id, &normalize_create_symbol(symbol))
                .await
                .map_err(|err| {
                    TraderApiError::internal(format!("failed to upsert symbol: {err}"))
                })?,
        );
    }
    Ok(TraderSymbolsResponse { symbols })
}

pub async fn suggest_symbols(
    database: &Database,
    trader_id: &str,
    request: SuggestTraderSymbolsRequest,
) -> Result<SuggestTraderSymbolsResponse, TraderApiError> {
    let trader = ensure_trader_exists(database, trader_id).await?;
    let max_symbols = request.max_symbols.unwrap_or(15).clamp(1, 50);
    let key = database
        .get_trader_openai_api_key(trader_id)
        .await
        .map_err(|err| TraderApiError::internal(format!("failed to load trader key: {err}")))?
        .filter(|key| !key.trim().is_empty() && key.trim() != "missing-key-add-in-trader-form")
        .or_else(|| env::var("CHAT_DEFAULT_OPENAI_API_KEY").ok())
        .or_else(|| env::var("OPENAI_API_KEY").ok())
        .ok_or_else(|| {
            TraderApiError::conflict("This trader does not have an API key configured.")
        })?;
    let data_sources = database
        .list_trader_data_sources(trader_id)
        .await
        .unwrap_or_default();
    let existing = database
        .list_trader_symbols(trader_id, None, None, None)
        .await
        .unwrap_or_default();
    let suggestions = call_symbol_suggestion_openai(
        &key,
        &trader,
        &data_sources,
        &existing,
        &request,
        max_symbols,
    )
    .await?;
    let mut saved = Vec::new();
    for mut suggestion in suggestions {
        suggestion.status = Some("candidate".to_string());
        suggestion.source = Some("ai".to_string());
        validate_create_symbol(&suggestion)?;
        saved.push(
            database
                .upsert_trader_symbol(trader_id, &normalize_create_symbol(suggestion))
                .await
                .map_err(|err| {
                    TraderApiError::internal(format!("failed to save suggested symbol: {err}"))
                })?,
        );
    }
    Ok(SuggestTraderSymbolsResponse { suggestions: saved })
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

fn validate_portfolio_proposal(
    request: &CreateTraderPortfolioProposalRequest,
) -> Result<(), TraderApiError> {
    if request.title.trim().is_empty() {
        return Err(TraderApiError::bad_request("title must be non-empty"));
    }
    if request.summary.trim().is_empty() {
        return Err(TraderApiError::bad_request("summary must be non-empty"));
    }
    if request.thesis.trim().is_empty() {
        return Err(TraderApiError::bad_request("thesis must be non-empty"));
    }
    validate_fit_score(request.confidence)?;
    if request.proposed_actions.is_empty() {
        return Err(TraderApiError::bad_request(
            "proposal must include at least one action",
        ));
    }
    for action in &request.proposed_actions {
        validate_portfolio_proposal_action(action)?;
    }
    Ok(())
}

fn validate_portfolio_proposal_action(
    action: &CreateTraderPortfolioProposalActionRequest,
) -> Result<(), TraderApiError> {
    match action.action_type.trim() {
        "buy" | "sell" | "hold" | "watch" | "activate_symbol" | "reject_symbol" | "reduce"
        | "increase" | "no_action" => {}
        _ => return Err(TraderApiError::bad_request("invalid action_type")),
    }
    if action.rationale.trim().is_empty() {
        return Err(TraderApiError::bad_request("rationale must be non-empty"));
    }
    if let Some(quantity) = action.quantity {
        if quantity <= 0.0 {
            return Err(TraderApiError::bad_request(
                "action quantity must be greater than 0",
            ));
        }
    }
    if let Some(side) = &action.side {
        match side.trim() {
            "buy" | "sell" => {}
            _ => return Err(TraderApiError::bad_request("side must be buy or sell")),
        }
    }
    validate_fit_score(action.confidence)?;
    Ok(())
}

fn validate_portfolio_proposal_status(status: &str) -> Result<(), TraderApiError> {
    match status.trim() {
        "proposed" | "accepted" | "rejected" | "superseded" | "executed" | "expired" => Ok(()),
        _ => Err(TraderApiError::bad_request("invalid proposal status")),
    }
}

fn normalize_create_symbol(mut request: CreateTraderSymbolRequest) -> CreateTraderSymbolRequest {
    request.symbol = request.symbol.trim().to_ascii_uppercase();
    request.asset_type = Some(
        request
            .asset_type
            .as_deref()
            .unwrap_or("stock")
            .trim()
            .to_ascii_lowercase(),
    );
    request.status = Some(
        request
            .status
            .as_deref()
            .unwrap_or("watching")
            .trim()
            .to_ascii_lowercase(),
    );
    request.source = Some(
        request
            .source
            .as_deref()
            .unwrap_or("manual")
            .trim()
            .to_ascii_lowercase(),
    );
    request
}

fn validate_create_symbol(request: &CreateTraderSymbolRequest) -> Result<(), TraderApiError> {
    if request.symbol.trim().is_empty() {
        return Err(TraderApiError::bad_request("symbol must be non-empty"));
    }
    validate_asset_type(request.asset_type.as_deref().unwrap_or("stock"))?;
    validate_symbol_status(request.status.as_deref().unwrap_or("watching"))?;
    validate_symbol_source(request.source.as_deref().unwrap_or("manual"))?;
    validate_fit_score(request.fit_score)?;
    Ok(())
}

fn validate_update_symbol(request: &UpdateTraderSymbolRequest) -> Result<(), TraderApiError> {
    if let Some(asset_type) = &request.asset_type {
        validate_asset_type(asset_type)?;
    }
    if let Some(status) = &request.status {
        validate_symbol_status(status)?;
    }
    validate_fit_score(request.fit_score)?;
    Ok(())
}

fn validate_asset_type(asset_type: &str) -> Result<(), TraderApiError> {
    match asset_type.trim().to_ascii_lowercase().as_str() {
        "stock" | "etf" | "index" | "crypto" | "other" => Ok(()),
        _ => Err(TraderApiError::bad_request("invalid asset_type")),
    }
}

fn validate_symbol_status(status: &str) -> Result<(), TraderApiError> {
    match status.trim().to_ascii_lowercase().as_str() {
        "watching" | "candidate" | "active" | "rejected" | "archived" => Ok(()),
        _ => Err(TraderApiError::bad_request("invalid status")),
    }
}

fn validate_symbol_source(source: &str) -> Result<(), TraderApiError> {
    match source.trim().to_ascii_lowercase().as_str() {
        "manual" | "ai" | "import" | "engine" => Ok(()),
        _ => Err(TraderApiError::bad_request("invalid source")),
    }
}

fn validate_fit_score(score: Option<f64>) -> Result<(), TraderApiError> {
    if let Some(score) = score {
        if !(0.0..=1.0).contains(&score) {
            return Err(TraderApiError::bad_request(
                "fit_score must be between 0 and 1",
            ));
        }
    }
    Ok(())
}

async fn call_symbol_suggestion_openai(
    api_key: &str,
    trader: &Trader,
    data_sources: &[models::data_sources::DataSource],
    existing: &[TraderSymbol],
    request: &SuggestTraderSymbolsRequest,
    max_symbols: i64,
) -> Result<Vec<CreateTraderSymbolRequest>, TraderApiError> {
    let model = env::var("TRADER_SYMBOL_SUGGESTION_MODEL")
        .or_else(|_| env::var("TRADER_CHAT_MODEL"))
        .or_else(|_| env::var("CHAT_COMMAND_MODEL"))
        .unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let prompt = format!(
        r#"Suggest a curated symbol universe for this AI trader.
Return JSON only in this shape: {{"suggestions":[{{"symbol":"SPY","asset_type":"etf","name":"...","sector":"...","industry":"...","thesis":"...","fit_score":0.92}}]}}

Rules:
- suggest stocks and ETFs that fit the trader perspective and goals
- prefer liquid, widely traded securities
- include a mix of core ETFs and individual stocks when appropriate
- avoid obscure tickers unless justified in thesis
- do not claim certainty
- maximum symbols: {max_symbols}
- include ETFs: {include_etfs}
- include stocks: {include_stocks}
- optional focus: {focus}

Trader:
name={name}
freedom_level={freedom}
perspective={perspective}

Assigned data sources:
{sources}

Existing symbols to avoid duplicating unless improving metadata:
{existing}
"#,
        max_symbols = max_symbols,
        include_etfs = request.include_etfs.unwrap_or(true),
        include_stocks = request.include_stocks.unwrap_or(true),
        focus = request.focus.as_deref().unwrap_or("none"),
        name = trader.name,
        freedom = trader.freedom_level,
        perspective = trader.fundamental_perspective,
        sources = if data_sources.is_empty() {
            "none".to_string()
        } else {
            data_sources
                .iter()
                .map(|source| format!("{} ({})", source.name, source.source_type))
                .collect::<Vec<_>>()
                .join(", ")
        },
        existing = if existing.is_empty() {
            "none".to_string()
        } else {
            existing
                .iter()
                .map(|symbol| symbol.symbol.clone())
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": "Return only valid JSON. Do not include markdown." },
            { "role": "user", "content": prompt }
        ],
        "response_format": { "type": "json_object" }
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| TraderApiError::internal(format!("failed to build OpenAI client: {err}")))?;
    let response: Value = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| TraderApiError::internal(format!("OpenAI request failed: {err}")))?
        .error_for_status()
        .map_err(|err| TraderApiError::internal(format!("OpenAI request failed: {err}")))?
        .json()
        .await
        .map_err(|err| TraderApiError::internal(format!("OpenAI response was invalid: {err}")))?;
    let content = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .ok_or_else(|| TraderApiError::internal("OpenAI returned an empty suggestion response"))?;
    let parsed: Value = serde_json::from_str(content).map_err(|err| {
        TraderApiError::internal(format!("OpenAI suggestion JSON was invalid: {err}"))
    })?;
    let values = parsed
        .get("suggestions")
        .and_then(Value::as_array)
        .ok_or_else(|| TraderApiError::internal("OpenAI response missing suggestions"))?;
    let mut output = Vec::new();
    for value in values.iter().take(max_symbols as usize) {
        let Some(symbol) = value.get("symbol").and_then(Value::as_str) else {
            continue;
        };
        output.push(CreateTraderSymbolRequest {
            symbol: symbol.to_string(),
            asset_type: value
                .get("asset_type")
                .and_then(Value::as_str)
                .map(str::to_string),
            name: value
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string),
            exchange: value
                .get("exchange")
                .and_then(Value::as_str)
                .map(str::to_string),
            sector: value
                .get("sector")
                .and_then(Value::as_str)
                .map(str::to_string),
            industry: value
                .get("industry")
                .and_then(Value::as_str)
                .map(str::to_string),
            notes: None,
            thesis: value
                .get("thesis")
                .and_then(Value::as_str)
                .map(str::to_string),
            fit_score: value.get("fit_score").and_then(Value::as_f64),
            status: Some("candidate".to_string()),
            source: Some("ai".to_string()),
        });
    }
    Ok(output)
}

fn map_paper_error(error: PaperApiError) -> TraderApiError {
    match error.kind {
        PaperErrorKind::BadRequest => TraderApiError::bad_request(error.message),
        PaperErrorKind::NotFound => TraderApiError::not_found(error.message),
        PaperErrorKind::Conflict => TraderApiError::conflict(error.message),
        PaperErrorKind::Internal => TraderApiError::internal(error.message),
    }
}
