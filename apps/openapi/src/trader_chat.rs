use std::{env, time::Duration};

use database::Database;
use models::trader::{
    TraderChatMessage, TraderChatRequest, TraderChatResponse, TraderTradeProposal,
};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraderChatErrorKind {
    BadRequest,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug)]
pub struct TraderChatError {
    pub kind: TraderChatErrorKind,
    pub message: String,
}

impl TraderChatError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: TraderChatErrorKind::BadRequest,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: TraderChatErrorKind::NotFound,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: TraderChatErrorKind::Conflict,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: TraderChatErrorKind::Internal,
            message: message.into(),
        }
    }
}

pub async fn chat(
    database: &Database,
    trader_id: &str,
    request: TraderChatRequest,
) -> Result<TraderChatResponse, TraderChatError> {
    let message = request.message.trim();
    if message.is_empty() {
        return Err(TraderChatError::bad_request("message must be non-empty"));
    }

    let trader = database
        .get_trader(trader_id)
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load trader: {err}")))?
        .ok_or_else(|| TraderChatError::not_found("trader not found"))?;

    let key = select_openai_key(database, trader_id).await?;
    let data_sources = database
        .list_trader_data_sources(trader_id)
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load data sources: {err}")))?;
    let tracked_symbols = database
        .list_trader_symbols(trader_id, None, None, None)
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load symbols: {err}")))?;
    let mut item_lines = Vec::new();
    for source in &data_sources {
        let items = database
            .list_data_source_items(&source.id, 5)
            .await
            .map_err(|err| {
                TraderChatError::internal(format!("failed to load source items: {err}"))
            })?;
        for item in items {
            item_lines.push(format!(
                "{} [{}]: {} ({})",
                source.name,
                source.source_type,
                item.title,
                item.published_at
                    .as_deref()
                    .unwrap_or(item.discovered_at.as_str())
            ));
        }
    }

    let events = database
        .list_trader_events(trader_id, 20)
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load events: {err}")))?;
    let proposals = database
        .list_trader_trade_proposals(trader_id)
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load proposals: {err}")))?;
    let runtime = database
        .get_trader_runtime_state(trader_id)
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load runtime state: {err}")))?;
    let orders = database
        .list_recent_paper_orders_for_trader(trader_id, 20)
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load orders: {err}")))?;
    let fills = database
        .list_recent_paper_fills_for_trader(trader_id, 20)
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load fills: {err}")))?;
    let active_proposal = database
        .get_active_trader_portfolio_proposal(trader_id)
        .await
        .map_err(|err| {
            TraderChatError::internal(format!("failed to load active portfolio proposal: {err}"))
        })?;
    let latest_proposal = if active_proposal.is_some() {
        active_proposal
    } else {
        database
            .get_latest_trader_portfolio_proposal(trader_id)
            .await
            .map_err(|err| {
                TraderChatError::internal(format!(
                    "failed to load latest portfolio proposal: {err}"
                ))
            })?
    };

    let system_prompt = build_system_prompt(
        &trader,
        &data_sources,
        &tracked_symbols,
        latest_proposal.as_ref(),
        &item_lines,
        &events,
        &proposals,
        runtime.as_ref(),
        &orders,
        &fills,
    );
    let reply = call_openai(
        &key,
        &system_prompt,
        request.conversation.unwrap_or_default(),
        message,
    )
    .await?;

    Ok(TraderChatResponse {
        reply,
        trader_id: trader.id,
        trader_name: trader.name,
        referenced_events: events.into_iter().take(5).map(|event| event.id).collect(),
        referenced_proposals: proposals
            .into_iter()
            .take(5)
            .map(|proposal| proposal.id)
            .collect(),
        referenced_orders: orders.into_iter().take(5).map(|order| order.id).collect(),
    })
}

async fn select_openai_key(
    database: &Database,
    trader_id: &str,
) -> Result<String, TraderChatError> {
    let trader_key = database
        .get_trader_openai_api_key(trader_id)
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load trader key: {err}")))?
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty() && key != "missing-key-add-in-trader-form");

    trader_key
        .or_else(|| env::var("CHAT_DEFAULT_OPENAI_API_KEY").ok())
        .or_else(|| env::var("OPENAI_API_KEY").ok())
        .ok_or_else(|| {
            TraderChatError::conflict("This trader does not have an API key configured.")
        })
}

fn build_system_prompt(
    trader: &models::trader::Trader,
    data_sources: &[models::data_sources::DataSource],
    tracked_symbols: &[models::trader::TraderSymbol],
    latest_proposal: Option<&models::trader::TraderPortfolioProposalDetail>,
    item_lines: &[String],
    events: &[models::trader::TraderEvent],
    proposals: &[TraderTradeProposal],
    runtime: Option<&models::trader::TraderRuntimeState>,
    orders: &[models::paper::PaperOrder],
    fills: &[models::paper::PaperFill],
) -> String {
    let sources = if data_sources.is_empty() {
        "No assigned data sources.".to_string()
    } else {
        data_sources
            .iter()
            .map(|source| {
                format!(
                    "{} ({}, {}, last checked {})",
                    source.name,
                    source.source_type,
                    if source.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    source.last_checked_at.as_deref().unwrap_or("never")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let events_text = if events.is_empty() {
        "No recent events.".to_string()
    } else {
        events
            .iter()
            .map(|event| {
                format!(
                    "{}: {} - {}",
                    event.created_at, event.event_type, event.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let symbols_text = if tracked_symbols.is_empty() {
        "No tracked symbols configured.".to_string()
    } else {
        tracked_symbols
            .iter()
            .map(|symbol| {
                format!(
                    "{} ({}) status={} source={} fit={:?}: {}",
                    symbol.symbol,
                    symbol.asset_type,
                    symbol.status,
                    symbol.source,
                    symbol.fit_score,
                    symbol
                        .thesis
                        .as_deref()
                        .or(symbol.notes.as_deref())
                        .unwrap_or("no thesis recorded")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let latest_proposal_text = latest_proposal
        .map(|detail| {
            let actions = detail
                .actions
                .iter()
                .map(|action| {
                    format!(
                        "{} {:?} {:?}: {} risk={}",
                        action.action_type,
                        action.symbol,
                        action.quantity,
                        action.rationale,
                        action.risk_decision.as_deref().unwrap_or("none")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{} status={} confidence={:?}\nSummary: {}\nThesis: {}\nActions:\n{}",
                detail.proposal.title,
                detail.proposal.status,
                detail.proposal.confidence,
                detail.proposal.summary,
                detail.proposal.thesis,
                if actions.is_empty() {
                    "none".to_string()
                } else {
                    actions
                }
            )
        })
        .unwrap_or_else(|| "No portfolio proposal yet.".to_string());
    let proposals_text = if proposals.is_empty() {
        "No recent trade proposals.".to_string()
    } else {
        proposals
            .iter()
            .take(10)
            .map(|proposal| {
                format!(
                    "{} {} {} {} status={} confidence={:?}: {}",
                    proposal.created_at,
                    proposal.side,
                    proposal.quantity,
                    proposal.symbol,
                    proposal.status,
                    proposal.confidence,
                    proposal.reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let runtime_text = runtime
        .map(|state| {
            format!(
                "engine={}; heartbeat={}; last evaluation={}; task={}; last error={}",
                state.engine_name.as_deref().unwrap_or("unknown"),
                state.last_heartbeat_at.as_deref().unwrap_or("none"),
                state.last_evaluation_at.as_deref().unwrap_or("none"),
                state.current_task.as_deref().unwrap_or("none"),
                state.last_error.as_deref().unwrap_or("none")
            )
        })
        .unwrap_or_else(|| "No runtime state recorded.".to_string());
    let orders_text = if orders.is_empty() {
        "No recent paper orders linked to this trader.".to_string()
    } else {
        orders
            .iter()
            .map(|order| {
                format!(
                    "{} {} {} {} status={} avg_fill={:?}",
                    order.created_at,
                    order.side,
                    order.quantity,
                    order.symbol,
                    order.status,
                    order.average_fill_price
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let fills_text = if fills.is_empty() {
        "No recent paper fills linked to this trader.".to_string()
    } else {
        fills
            .iter()
            .map(|fill| {
                format!(
                    "{} {} {} {} @ {} notional {}",
                    fill.created_at,
                    fill.side,
                    fill.quantity,
                    fill.symbol,
                    fill.price,
                    fill.notional
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"You are an AI trader named {name} inside the Desk trading application.

Your fundamental perspective:
{perspective}

Your freedom level is {freedom_level}:
- analyst: you may analyze and recommend, but you cannot trade
- junior_trader: you may propose trades for review through the engine workflow, but cannot execute them
- senior_trader: you may execute paper trades only through the engine and risk system

Status: {status}
Default paper account: {paper_account}

You must explain your reasoning clearly.
You must not claim to have placed a trade unless it appears in provided events/orders.
You must not promise real-money trading.
You must respect risk controls.
Direct chat is conversational and explanatory only. Never submit, place, execute, or approve trades from chat. If asked to trade, explain that execution can only happen through the engine/risk workflow.
Use first person as {name}. Base answers only on provided context. If context is missing, say so.

Assigned data sources:
{sources}

Tracked symbol universe:
{symbols}

Active/latest portfolio proposal:
{latest_proposal}

Recent data source items:
{items}

Runtime state:
{runtime}

Recent trader events:
{events}

Recent trade proposals:
{proposals}

Recent trader paper orders:
{orders}

Recent trader paper fills:
{fills}
"#,
        name = trader.name,
        perspective = trader.fundamental_perspective,
        freedom_level = trader.freedom_level,
        status = trader.status,
        paper_account = trader.default_paper_account_id.as_deref().unwrap_or("none"),
        sources = sources,
        symbols = symbols_text,
        latest_proposal = latest_proposal_text,
        items = if item_lines.is_empty() {
            "No recent data source items.".to_string()
        } else {
            item_lines.join("\n")
        },
        runtime = runtime_text,
        events = events_text,
        proposals = proposals_text,
        orders = orders_text,
        fills = fills_text
    )
}

async fn call_openai(
    api_key: &str,
    system_prompt: &str,
    conversation: Vec<TraderChatMessage>,
    message: &str,
) -> Result<String, TraderChatError> {
    let model = env::var("TRADER_CHAT_MODEL")
        .or_else(|_| env::var("CHAT_COMMAND_MODEL"))
        .unwrap_or_else(|_| "gpt-5.2".to_string());
    let mut messages = vec![json!({ "role": "system", "content": system_prompt })];
    for entry in conversation.into_iter().take(20) {
        let role = match entry.role.as_str() {
            "assistant" => "assistant",
            "user" => "user",
            _ => continue,
        };
        let content = entry.content.trim();
        if content.is_empty() {
            continue;
        }
        messages.push(json!({ "role": role, "content": content }));
    }
    messages.push(json!({ "role": "user", "content": message }));

    let body = json!({
        "model": model,
        "messages": messages
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| {
            TraderChatError::internal(format!("failed to build OpenAI client: {err}"))
        })?;
    let response: Value = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| TraderChatError::internal(format!("OpenAI request failed: {err}")))?
        .error_for_status()
        .map_err(|err| TraderChatError::internal(format!("OpenAI request failed: {err}")))?
        .json()
        .await
        .map_err(|err| TraderChatError::internal(format!("OpenAI response was invalid: {err}")))?;

    response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|reply| !reply.is_empty())
        .map(str::to_string)
        .ok_or_else(|| TraderChatError::internal("OpenAI returned an empty trader chat response"))
}
