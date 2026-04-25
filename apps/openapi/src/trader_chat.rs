use std::{env, time::Duration};

use database::Database;
use models::trader::{
    CreateTraderPortfolioProposalActionRequest, CreateTraderPortfolioProposalRequest,
    TraderChatAction, TraderChatMessage, TraderChatRequest, TraderChatResponse,
    TraderTradeProposal,
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
    let memories = database
        .search_trader_memories(trader_id, message, Some(5))
        .await
        .map_err(|err| TraderChatError::internal(format!("failed to load memories: {err}")))?;

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
        &memories.memories,
    );
    let conversation = request.conversation.unwrap_or_default();
    let mut reply = match call_openai(&key, &system_prompt, conversation.clone(), message).await {
        Ok(reply) => reply,
        Err(err) if should_create_or_update_portfolio_proposal(message, &conversation, "") => {
            format!(
                "I could not complete the model-backed chat response: {}. I saved a conservative draft proposal from your message instead, with all actions marked as review-only and no orders submitted.",
                err.message
            )
        }
        Err(err) => return Err(err),
    };
    let mut actions = Vec::new();
    if should_create_or_update_portfolio_proposal(message, &conversation, &reply) {
        let proposal_request = if should_use_direct_proposal_fallback(message) {
            Some(fallback_portfolio_proposal(
                &trader,
                &tracked_symbols,
                &conversation,
                message,
                &reply,
            ))
        } else {
            let extracted = extract_portfolio_proposal(
                &key,
                &trader,
                &tracked_symbols,
                latest_proposal.as_ref(),
                &conversation,
                message,
                &reply,
            )
            .await;
            match extracted {
                Ok(proposal_request) => proposal_request,
                Err(err) => {
                    reply.push_str(&format!(
                        "\n\nI drafted the proposal in chat, but the structured extraction call failed: {}. I saved a conservative draft proposal from the conversation instead.",
                        err.message
                    ));
                    Some(fallback_portfolio_proposal(
                        &trader,
                        &tracked_symbols,
                        &conversation,
                        message,
                        &reply,
                    ))
                }
            }
        };

        if let Some(proposal_request) = proposal_request {
            let detail =
                crate::traders::create_portfolio_proposal(database, trader_id, proposal_request)
                    .await
                    .map_err(|err| {
                        TraderChatError::internal(format!(
                            "failed to save portfolio proposal from chat: {}",
                            err.message
                        ))
                    })?;
            actions.push(TraderChatAction {
                r#type: "portfolio_proposal_created".to_string(),
                entity_id: Some(detail.proposal.id),
                title: Some(detail.proposal.title),
                status: Some(detail.proposal.status),
            });
        }
    }

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
        actions,
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
    memories: &[models::channels::TraderMemory],
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
    let memories_text = if memories.is_empty() {
        "No relevant trader memories.".to_string()
    } else {
        memories
            .iter()
            .map(|memory| format!("{}: {}", memory.topic, memory.summary))
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
Direct chat may create or revise portfolio proposal drafts when the user asks you to do that. Portfolio proposals are review artifacts only.
Never submit, place, execute, or approve trades from chat. If asked to trade, explain that execution can only happen through the engine/risk workflow.
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

Relevant trader memories:
{memories}
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
        fills = fills_text,
        memories = memories_text
    )
}

fn should_create_or_update_portfolio_proposal(
    message: &str,
    conversation: &[TraderChatMessage],
    reply: &str,
) -> bool {
    let combined = format!(
        "{}\n{}\n{}",
        conversation
            .iter()
            .rev()
            .take(6)
            .map(|entry| entry.content.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        message,
        reply
    )
    .to_lowercase();
    let proposal_intent = combined.contains("proposal")
        || combined.contains("portfolio plan")
        || combined.contains("draft")
        || combined.contains("lock the proposal")
        || combined.contains("finalize");
    let create_or_update = combined.contains("create")
        || combined.contains("new proposal")
        || combined.contains("update")
        || combined.contains("revise")
        || combined.contains("lock")
        || combined.contains("finalize")
        || combined.contains("got it");
    proposal_intent && create_or_update
}

fn should_use_direct_proposal_fallback(message: &str) -> bool {
    let lower = message.to_lowercase();
    message.len() > 1200
        && (lower.contains("new portfolio proposal")
            || lower.contains("status: proposed")
            || lower.contains("universe:")
            || lower.contains("risk controls")
            || lower.contains("execution rules"))
}

async fn extract_portfolio_proposal(
    api_key: &str,
    trader: &models::trader::Trader,
    tracked_symbols: &[models::trader::TraderSymbol],
    latest_proposal: Option<&models::trader::TraderPortfolioProposalDetail>,
    conversation: &[TraderChatMessage],
    message: &str,
    reply: &str,
) -> Result<Option<CreateTraderPortfolioProposalRequest>, TraderChatError> {
    let model = env::var("TRADER_CHAT_MODEL")
        .or_else(|_| env::var("CHAT_COMMAND_MODEL"))
        .unwrap_or_else(|_| "gpt-5.2".to_string());
    let transcript = conversation
        .iter()
        .rev()
        .take(12)
        .rev()
        .map(|entry| format!("{}: {}", entry.role, entry.content))
        .collect::<Vec<_>>()
        .join("\n");
    let latest_text = latest_proposal
        .map(|detail| {
            format!(
                "{} status={} summary={} thesis={}",
                detail.proposal.title,
                detail.proposal.status,
                detail.proposal.summary,
                detail.proposal.thesis
            )
        })
        .unwrap_or_else(|| "No latest proposal.".to_string());
    let symbol_text = if tracked_symbols.is_empty() {
        "No tracked symbols.".to_string()
    } else {
        tracked_symbols
            .iter()
            .map(|symbol| symbol.symbol.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let prompt = format!(
        r#"Extract a Desk portfolio proposal draft from this Trader chat.

Trader: {name}
Perspective: {perspective}
Tracked symbols: {symbols}
Latest proposal: {latest}

Conversation:
{transcript}

Current user message:
{message}

Trader reply:
{reply}

Return only JSON. If there is not enough concrete proposal content to save, return {{"should_create":false}}.
If there is enough content, return:
{{
  "should_create": true,
  "proposal": {{
    "trader_id": "{trader_id}",
    "paper_account_id": null,
    "title": "short title",
    "summary": "clear summary",
    "thesis": "why this plan exists",
    "confidence": 0.5,
    "expected_duration_seconds": 2419200,
    "source_snapshot_json": null,
    "risk_snapshot_json": "{{\"chat_created\":true}}",
    "market_snapshot_json": null,
    "market_basis_json": null,
    "invalidation_conditions_json": "{{\"conditions\":[\"Missing exact entry/stop levels should be filled before execution.\"]}}",
    "change_thresholds_json": null,
    "replacement_reason": "Created or revised from direct Trader chat.",
    "proposed_actions": [
      {{
        "symbol": "SPY",
        "action_type": "watch",
        "side": null,
        "quantity": null,
        "order_type": null,
        "entry_price": null,
        "exit_price": null,
        "limit_price": null,
        "stop_price": null,
        "expected_duration_seconds": 2419200,
        "enact_by": null,
        "market_price_at_creation": null,
        "rationale": "rationale from the chat",
        "confidence": 0.5,
        "risk_decision": "chat draft only; no order submitted"
      }}
    ]
  }}
}}

Rules:
- This is a proposal artifact only. Do not create executed/accepted status.
- Do not include an action that claims an order was placed.
- Use action_type watch, buy, sell, hold, reduce, increase, or no_action.
- Use buy/sell actions only as proposed review actions; no execution.
- If exact quantity is unknown because the user specified percent sizing, leave quantity null and put the percent cap in rationale/risk JSON.
- Include at least one proposed action when should_create is true.
"#,
        name = trader.name,
        perspective = trader.fundamental_perspective,
        symbols = symbol_text,
        latest = latest_text,
        transcript = transcript,
        message = message,
        reply = reply,
        trader_id = trader.id,
    );
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": "You convert Trader chat into safe Desk portfolio proposal JSON. Return only JSON." },
            { "role": "user", "content": prompt }
        ]
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
    let Some(text) = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    let Some(value) = parse_json_object(text) else {
        return Ok(None);
    };
    if !value
        .get("should_create")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let Some(proposal) = value.get("proposal").cloned() else {
        return Ok(None);
    };
    let mut request = serde_json::from_value::<CreateTraderPortfolioProposalRequest>(proposal)
        .map_err(|err| {
            TraderChatError::internal(format!("OpenAI proposal JSON was invalid: {err}"))
        })?;
    request.trader_id = trader.id.clone();
    if request.title.trim().is_empty()
        || request.summary.trim().is_empty()
        || request.thesis.trim().is_empty()
        || request.proposed_actions.is_empty()
    {
        return Ok(None);
    }
    Ok(Some(request))
}

fn fallback_portfolio_proposal(
    trader: &models::trader::Trader,
    tracked_symbols: &[models::trader::TraderSymbol],
    conversation: &[TraderChatMessage],
    message: &str,
    reply: &str,
) -> CreateTraderPortfolioProposalRequest {
    let combined = format!(
        "{}\n{}\n{}",
        conversation
            .iter()
            .rev()
            .take(8)
            .map(|entry| entry.content.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        message,
        reply
    );
    let combined_upper = combined.to_ascii_uppercase();
    let mentioned_symbols = tracked_symbols
        .iter()
        .filter_map(|symbol| {
            let symbol_text = symbol.symbol.trim().to_ascii_uppercase();
            if symbol_text.is_empty() {
                return None;
            }
            if combined_upper
                .split(|ch: char| !ch.is_ascii_alphanumeric())
                .any(|token| token == symbol_text)
            {
                Some(symbol_text)
            } else {
                None
            }
        })
        .take(20)
        .collect::<Vec<_>>();
    let duration_seconds = if combined.to_lowercase().contains("4 week") {
        Some(60 * 60 * 24 * 28)
    } else {
        Some(60 * 60 * 24 * 30)
    };
    let summary = summarize_text(reply, 420);
    let sizing_note = extract_percent_note(&combined)
        .map(|note| format!(" Position sizing note: {note}."))
        .unwrap_or_default();
    let risk_snapshot = json!({
        "chat_created": true,
        "fallback_extraction": true,
        "note": "Structured extraction failed, so this proposal was saved conservatively from chat text.",
        "sizing": extract_percent_note(&combined)
    })
    .to_string();
    let invalidation = json!({
        "conditions": [
            "Exact entries, exits, and stops must be reviewed before any execution workflow.",
            "Do not execute from chat; use proposal review and risk controls."
        ]
    })
    .to_string();
    let actions = if mentioned_symbols.is_empty() {
        vec![CreateTraderPortfolioProposalActionRequest {
            symbol: None,
            action_type: "no_action".to_string(),
            side: None,
            quantity: None,
            order_type: None,
            entry_price: None,
            exit_price: None,
            limit_price: None,
            stop_price: None,
            expected_duration_seconds: duration_seconds,
            enact_by: None,
            market_price_at_creation: None,
            rationale: format!(
                "Chat-created draft proposal. No specific tracked symbol could be safely extracted.{}",
                sizing_note
            ),
            confidence: Some(0.35),
            risk_decision: Some("chat draft only; no order submitted".to_string()),
        }]
    } else {
        mentioned_symbols
            .into_iter()
            .map(|symbol| CreateTraderPortfolioProposalActionRequest {
                symbol: Some(symbol.clone()),
                action_type: "watch".to_string(),
                side: None,
                quantity: None,
                order_type: None,
                entry_price: None,
                exit_price: None,
                limit_price: None,
                stop_price: None,
                expected_duration_seconds: duration_seconds,
                enact_by: None,
                market_price_at_creation: None,
                rationale: format!(
                    "{symbol} was included in the user's chat-directed proposal universe. Keep as watch/review until entries, exits, stops, and risk are finalized.{}",
                    sizing_note
                ),
                confidence: Some(0.45),
                risk_decision: Some("chat draft only; no order submitted".to_string()),
            })
            .collect()
    };

    CreateTraderPortfolioProposalRequest {
        trader_id: trader.id.clone(),
        paper_account_id: trader.default_paper_account_id.clone(),
        title: title_from_chat(&combined),
        summary,
        thesis: format!(
            "Created from a direct Trader chat request. The draft should be reviewed against current market data, source context, and risk controls before it becomes an active plan. Trader perspective: {}",
            summarize_text(&trader.fundamental_perspective, 280)
        ),
        confidence: Some(0.45),
        expected_duration_seconds: duration_seconds,
        proposed_actions: actions,
        source_snapshot_json: None,
        risk_snapshot_json: Some(risk_snapshot),
        market_snapshot_json: None,
        market_basis_json: None,
        invalidation_conditions_json: Some(invalidation),
        change_thresholds_json: None,
        replacement_reason: Some("Created from direct Trader chat.".to_string()),
    }
}

fn title_from_chat(text: &str) -> String {
    let lower = text.to_lowercase();
    if lower.contains("4 week") {
        "4-Week Chat-Drafted Portfolio Proposal".to_string()
    } else if lower.contains("swing") {
        "Chat-Drafted Swing Portfolio Proposal".to_string()
    } else {
        "Chat-Drafted Portfolio Proposal".to_string()
    }
}

fn summarize_text(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        return compact;
    }
    let mut output = compact.chars().take(max_chars).collect::<String>();
    output.push_str("...");
    output
}

fn extract_percent_note(text: &str) -> Option<String> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    for index in 0..words.len() {
        let word = words[index]
            .trim_matches(|ch: char| matches!(ch, ',' | '.' | ':' | ';' | ')' | '(' | '"' | '\''));
        if word.ends_with('%')
            && word[..word.len().saturating_sub(1)]
                .chars()
                .all(|ch| ch.is_ascii_digit() || ch == '.')
        {
            let start = index.saturating_sub(3);
            let end = (index + 4).min(words.len());
            return Some(words[start..end].join(" "));
        }
    }
    None
}

fn parse_json_object(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text).ok().or_else(|| {
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        serde_json::from_str::<Value>(&text[start..=end]).ok()
    })
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
