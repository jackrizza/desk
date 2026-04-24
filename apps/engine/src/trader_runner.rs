use anyhow::Result;
use chrono::Utc;
use tracing::{info, warn};

use crate::{
    config::EngineConfig,
    trader_client::TraderClient,
    trader_types::{
        CreateTraderEventRequest, CreateTraderTradeProposalRequest, EngineRunnableTrader,
        OpenAiChatRequest, OpenAiMessage, OpenAiResponseFormat, TraderAiDecision,
        UpsertTraderRuntimeStateRequest,
    },
    types::SubmitPaperOrderRequest,
};

pub async fn run_traders(config: &EngineConfig, client: &TraderClient) -> Result<()> {
    let traders = client.fetch_running_traders().await?;
    info!(
        trader_count = traders.len(),
        "received engine trader configs"
    );

    for trader in traders {
        if let Err(err) = run_trader(config, client, &trader).await {
            warn!(
                trader_id = %trader.id,
                trader_name = %trader.name,
                error = %err,
                "trader evaluation failed"
            );
            let now = Utc::now().to_rfc3339();
            let _ = client
                .upsert_runtime_state(
                    &trader.id,
                    &UpsertTraderRuntimeStateRequest {
                        engine_name: Some(config.engine_name.clone()),
                        last_heartbeat_at: Some(now.clone()),
                        last_evaluation_at: Some(now),
                        last_error: Some(err.to_string()),
                        current_task: Some("error".to_string()),
                    },
                )
                .await;
        }
    }

    Ok(())
}

async fn run_trader(
    config: &EngineConfig,
    client: &TraderClient,
    trader: &EngineRunnableTrader,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    client
        .upsert_runtime_state(
            &trader.id,
            &UpsertTraderRuntimeStateRequest {
                engine_name: Some(config.engine_name.clone()),
                last_heartbeat_at: Some(now.clone()),
                last_evaluation_at: None,
                last_error: None,
                current_task: Some("evaluating".to_string()),
            },
        )
        .await?;

    let decision = client
        .ask_openai(&trader.openai_api_key, &build_openai_request(trader))
        .await
        .unwrap_or_else(|err| TraderAiDecision {
            action: "hold".to_string(),
            symbol: None,
            side: None,
            quantity: None,
            reason: format!("Trader held because AI evaluation failed: {err}"),
            confidence: None,
        });

    enforce_decision(client, trader, decision).await?;
    let finished_at = Utc::now().to_rfc3339();
    client
        .upsert_runtime_state(
            &trader.id,
            &UpsertTraderRuntimeStateRequest {
                engine_name: Some(config.engine_name.clone()),
                last_heartbeat_at: Some(finished_at.clone()),
                last_evaluation_at: Some(finished_at),
                last_error: None,
                current_task: Some("idle".to_string()),
            },
        )
        .await?;

    Ok(())
}

fn build_openai_request(trader: &EngineRunnableTrader) -> OpenAiChatRequest {
    let source_names = trader
        .info_sources
        .iter()
        .filter(|source| source.enabled)
        .map(|source| format!("{}:{}", source.source_type, source.name))
        .collect::<Vec<_>>()
        .join(", ");
    let assigned_sources = trader
        .data_sources
        .iter()
        .map(|source| format!("{}:{}", source.source_type, source.name))
        .collect::<Vec<_>>()
        .join(", ");
    let paper_account = trader.default_paper_account_id.as_deref().unwrap_or("none");
    let prompt = format!(
        "You are an autonomous paper-trading assistant named {name}. Perspective: {perspective}. Freedom level: {freedom}. Paper account: {paper_account}. Info source placeholders: {sources}. Assigned data sources: {assigned_sources}. Return strict JSON with action hold, propose_trade, or trade; optional symbol, side, quantity; reason; confidence. Use conservative risk-aware behavior.",
        name = trader.name,
        perspective = trader.fundamental_perspective,
        freedom = trader.freedom_level,
        sources = if source_names.is_empty() {
            "none"
        } else {
            &source_names
        },
        assigned_sources = if assigned_sources.is_empty() {
            "none"
        } else {
            &assigned_sources
        },
    );

    OpenAiChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![
            OpenAiMessage {
                role: "system".to_string(),
                content: "Return only JSON. Never suggest live broker execution.".to_string(),
            },
            OpenAiMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        response_format: OpenAiResponseFormat {
            r#type: "json_object".to_string(),
        },
    }
}

async fn enforce_decision(
    client: &TraderClient,
    trader: &EngineRunnableTrader,
    decision: TraderAiDecision,
) -> Result<()> {
    match trader.freedom_level.as_str() {
        "analyst" => {
            client
                .create_event(
                    &trader.id,
                    &CreateTraderEventRequest {
                        event_type: "trader_recommendation".to_string(),
                        message: decision.reason,
                        payload: serde_json::to_string(&serde_json::json!({
                            "action": decision.action,
                            "symbol": decision.symbol,
                            "side": decision.side,
                            "quantity": decision.quantity,
                            "confidence": decision.confidence,
                        }))
                        .ok(),
                    },
                )
                .await?;
        }
        "junior_trader" => {
            if decision.action == "propose_trade" || decision.action == "trade" {
                let Some(request) = proposal_from_decision(decision) else {
                    return Ok(());
                };
                let proposal = client.create_trade_proposal(&trader.id, &request).await?;
                client
                    .create_event(
                        &trader.id,
                        &CreateTraderEventRequest {
                            event_type: "trade_proposal_created".to_string(),
                            message: format!("Created proposal {}", proposal.id),
                            payload: None,
                        },
                    )
                    .await?;
            } else {
                client
                    .create_event(
                        &trader.id,
                        &CreateTraderEventRequest {
                            event_type: "trader_hold".to_string(),
                            message: decision.reason,
                            payload: None,
                        },
                    )
                    .await?;
            }
        }
        "senior_trader" => {
            if decision.action != "trade" {
                client
                    .create_event(
                        &trader.id,
                        &CreateTraderEventRequest {
                            event_type: "trader_hold".to_string(),
                            message: decision.reason,
                            payload: None,
                        },
                    )
                    .await?;
                return Ok(());
            }

            let Some(account_id) = trader.default_paper_account_id.clone() else {
                client
                    .create_event(
                        &trader.id,
                        &CreateTraderEventRequest {
                            event_type: "trade_blocked_no_paper_account".to_string(),
                            message: "Senior trader cannot trade without a selected paper account"
                                .to_string(),
                            payload: None,
                        },
                    )
                    .await?;
                return Ok(());
            };
            let Some(order) = order_from_decision(account_id, &trader.id, decision) else {
                return Ok(());
            };

            // TODO: add trader-specific risk configuration. V1 relies on paper account
            // cash/position checks and never submits live broker orders.
            match client.submit_paper_order(&order).await {
                Ok(result) => {
                    client
                        .create_event(
                            &trader.id,
                            &CreateTraderEventRequest {
                                event_type: "paper_order_submitted".to_string(),
                                message: format!("Submitted paper order {}", result.order.id),
                                payload: None,
                            },
                        )
                        .await?;
                }
                Err(err) => {
                    client
                        .create_event(
                            &trader.id,
                            &CreateTraderEventRequest {
                                event_type: "paper_order_blocked".to_string(),
                                message: format!("Paper order blocked: {err}"),
                                payload: None,
                            },
                        )
                        .await?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn proposal_from_decision(decision: TraderAiDecision) -> Option<CreateTraderTradeProposalRequest> {
    Some(CreateTraderTradeProposalRequest {
        symbol: decision.symbol?,
        side: decision.side?,
        quantity: decision.quantity?,
        order_type: Some("market".to_string()),
        reason: decision.reason,
        confidence: decision.confidence,
    })
}

fn order_from_decision(
    account_id: String,
    trader_id: &str,
    decision: TraderAiDecision,
) -> Option<SubmitPaperOrderRequest> {
    Some(SubmitPaperOrderRequest {
        account_id,
        symbol: decision.symbol?,
        side: decision.side?,
        order_type: "market".to_string(),
        quantity: decision.quantity?,
        requested_price: None,
        source: Some("trader".to_string()),
        trader_id: Some(trader_id.to_string()),
        strategy_id: None,
        signal_id: None,
        proposal_id: None,
    })
}
