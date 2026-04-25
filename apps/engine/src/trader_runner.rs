use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use anyhow::Result;
use cache::Cache;
use chrono::Utc;
use tracing::{info, warn};

use crate::{
    channels,
    config::EngineConfig,
    plan_monitor,
    trader_client::TraderClient,
    trader_types::{
        CreateTraderEventRequest, CreateTraderPortfolioProposalActionRequest,
        CreateTraderPortfolioProposalRequest, CreateTraderTradeProposalRequest,
        EngineRunnableTrader, OpenAiChatRequest, OpenAiMessage, OpenAiResponseFormat,
        TraderAiDecision, TraderAiPortfolioProposal, UpsertTraderRuntimeStateRequest,
    },
    types::SubmitPaperOrderRequest,
};

static LAST_PROPOSAL_AT: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();

pub async fn run_traders(
    config: &EngineConfig,
    client: &TraderClient,
    cache: Arc<Cache>,
) -> Result<()> {
    let traders = client.fetch_running_traders().await?;
    info!(
        trader_count = traders.len(),
        "received engine trader configs"
    );

    for trader in traders {
        if let Err(err) = run_trader(config, client, &trader, cache.clone()).await {
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
    cache: Arc<Cache>,
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

    let channel_insight = channels::inspect_trader_channels(config, client, trader).await;
    if let Some(insight) = &channel_insight {
        info!(
            trader_id = %trader.id,
            latest_question_answered = insight.latest_question_answered,
            "loaded trader channel context"
        );
    }
    if let Err(err) = channels::maybe_post_trader_answer_followup(config, client, trader).await {
        warn!(
            trader_id = %trader.id,
            error = %err,
            "failed to post trader channel follow-up"
        );
    }
    if let Err(err) = channels::maybe_post_trader_user_reply(config, client, trader).await {
        warn!(
            trader_id = %trader.id,
            error = %err,
            "failed to post trader reply to user channel mention"
        );
    }

    maybe_create_portfolio_proposal(
        config,
        client,
        trader,
        cache.clone(),
        channel_insight.as_ref(),
    )
    .await?;

    let evaluable_symbols = trader
        .tracked_symbols
        .iter()
        .filter(|symbol| symbol.status == "active" || symbol.status == "watching")
        .collect::<Vec<_>>();
    if evaluable_symbols.is_empty() {
        client
            .create_event(
                &trader.id,
                &CreateTraderEventRequest {
                    event_type: "no_tracked_symbols".to_string(),
                    message: "No tracked symbols configured.".to_string(),
                    payload: None,
                },
            )
            .await?;
        channels::post_no_symbols_message(config, client, trader).await;
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
        return Ok(());
    }

    let decision = client
        .ask_openai(
            &trader.openai_api_key,
            &build_openai_request(trader, channel_insight.as_ref()),
        )
        .await
        .unwrap_or_else(|err| TraderAiDecision {
            action: "hold".to_string(),
            symbol: None,
            side: None,
            quantity: None,
            reason: format!("Trader held because AI evaluation failed: {err}"),
            confidence: None,
        });

    if let Err(err) = channels::maybe_post_trader_message(config, client, trader, &decision).await {
        warn!(
            trader_id = %trader.id,
            error = %err,
            "failed to post trader channel message"
        );
    }
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

async fn maybe_create_portfolio_proposal(
    config: &EngineConfig,
    client: &TraderClient,
    trader: &EngineRunnableTrader,
    cache: Arc<Cache>,
    channel_insight: Option<&channels::TraderChannelInsight>,
) -> Result<()> {
    if !proposal_due(&trader.id, config.trader_proposal_interval_seconds) {
        return Ok(());
    }

    if let Some(active_plan) = client.fetch_active_portfolio_proposal(&trader.id).await? {
        let current_prices = collect_current_prices(trader, cache.clone()).await;
        let decision = plan_monitor::evaluate_active_plan(trader, &active_plan, &current_prices);
        if !decision.should_replace {
            client
                .create_event(
                    &trader.id,
                    &CreateTraderEventRequest {
                        event_type: "active_plan_held".to_string(),
                        message: decision.reason,
                        payload: serde_json::to_string(&serde_json::json!({
                            "proposal_id": active_plan.proposal.id
                        }))
                        .ok(),
                    },
                )
                .await?;
            return Ok(());
        }

        client
            .create_event(
                &trader.id,
                &CreateTraderEventRequest {
                    event_type: "active_plan_change_detected".to_string(),
                    message: decision.reason.clone(),
                    payload: serde_json::to_string(&serde_json::json!({
                        "proposal_id": active_plan.proposal.id,
                        "material_changes": decision.material_changes,
                    }))
                    .ok(),
                },
            )
            .await?;
        let _ = client
            .review_portfolio_proposal(
                &trader.id,
                &active_plan.proposal.id,
                "expired",
                Some(&decision.reason),
            )
            .await;
    }

    let current_prices = collect_current_prices(trader, cache.clone()).await;
    let request = if trader
        .tracked_symbols
        .iter()
        .filter(|symbol| {
            symbol.status == "active" || symbol.status == "watching" || symbol.status == "candidate"
        })
        .count()
        == 0
    {
        no_symbol_proposal_request(trader)
    } else {
        client
            .ask_openai_for_portfolio_proposal(
                &trader.openai_api_key,
                &build_portfolio_proposal_openai_request(trader, channel_insight),
            )
            .await
            .map(|proposal| proposal_request_from_ai(trader, proposal, None, &current_prices))
            .unwrap_or_else(|err| fallback_proposal_request(trader, &err.to_string()))
    };

    let proposal = client
        .create_portfolio_proposal(&trader.id, &request)
        .await?;
    client
        .create_event(
            &trader.id,
            &CreateTraderEventRequest {
                event_type: "proposal_created".to_string(),
                message: format!("Created portfolio proposal {}", proposal.proposal.title),
                payload: serde_json::to_string(&serde_json::json!({
                    "proposal_id": proposal.proposal.id
                }))
                .ok(),
            },
        )
        .await?;
    Ok(())
}

fn proposal_due(trader_id: &str, interval_seconds: u64) -> bool {
    let now = Utc::now().timestamp();
    let interval = interval_seconds.max(1) as i64;
    let state = LAST_PROPOSAL_AT.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut guard) = state.lock() else {
        return true;
    };
    let due = guard
        .get(trader_id)
        .map(|last| now - *last >= interval)
        .unwrap_or(true);
    if due {
        guard.insert(trader_id.to_string(), now);
    }
    due
}

async fn collect_current_prices(
    trader: &EngineRunnableTrader,
    cache: Arc<Cache>,
) -> HashMap<String, f64> {
    let mut prices = HashMap::new();
    for symbol in trader
        .tracked_symbols
        .iter()
        .filter(|symbol| symbol.status == "active" || symbol.status == "watching")
        .map(|symbol| symbol.symbol.clone())
    {
        let cache_key = format!("{symbol}_1d_1m_false");
        if let Ok(data) = cache.check_cache(&cache_key).await {
            if let Some(price) = data
                .stock_data
                .iter()
                .rev()
                .find_map(|entry| entry.close.parse::<f64>().ok())
            {
                prices.insert(symbol, price);
            }
        }
    }
    prices
}

fn build_portfolio_proposal_openai_request(
    trader: &EngineRunnableTrader,
    channel_insight: Option<&channels::TraderChannelInsight>,
) -> OpenAiChatRequest {
    let symbols = trader
        .tracked_symbols
        .iter()
        .filter(|symbol| {
            symbol.status == "active" || symbol.status == "watching" || symbol.status == "candidate"
        })
        .map(|symbol| {
            format!(
                "{} ({}, status={}, fit={:?}): {}",
                symbol.symbol,
                symbol.asset_type,
                symbol.status,
                symbol.fit_score,
                symbol.thesis.as_deref().unwrap_or("no thesis")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let data_sources = trader
        .data_sources
        .iter()
        .map(|source| format!("{} ({})", source.name, source.source_type))
        .collect::<Vec<_>>()
        .join(", ");
    let channel_context = channel_insight
        .map(|insight| insight.summary.as_str())
        .unwrap_or("Channels disabled or no channel context loaded.");
    let prompt = format!(
        r#"Create the latest portfolio proposal for this AI trader.
Return strict JSON only:
{{
  "title": "...",
  "summary": "...",
  "thesis": "...",
  "confidence": 0.72,
  "expected_duration_seconds": 86400,
  "market_basis": {{"generated_at":"...","symbols":[]}},
  "invalidation_conditions": ["Material thesis break"],
  "change_thresholds": {{"material_price_move_percent":3.0,"material_news_change":true,"risk_limit_change":true}},
  "actions": [
    {{
      "symbol": "SPY",
      "action_type": "hold",
      "side": null,
      "quantity": null,
      "order_type": null,
      "entry_price": null,
      "exit_price": null,
      "limit_price": null,
      "stop_price": null,
      "expected_duration_seconds": 86400,
      "enact_by": null,
      "market_price_at_creation": null,
      "rationale": "...",
      "confidence": 0.68
    }}
  ]
}}

Rules:
- Always return at least one action.
- If no trade is appropriate, use no_action or hold.
- Do not invent executed trades.
- Do not recommend symbols outside the tracked universe unless action_type is watch.
- Include expected duration, invalidation conditions, and change thresholds.
- Buy/sell/reduce/increase actions should include entry_price, exit_price, and limit_price when possible.
- Analyst and junior traders create proposals only.
- Senior traders still only paper trade through engine/risk controls, not this proposal.
- Respect risk controls and prefer conservative quantities.

Trader: {name}
Freedom: {freedom}
Perspective: {perspective}
Paper account: {paper_account}
Assigned data sources: {data_sources}
Channel context:
{channel_context}
Tracked symbols:
{symbols}
"#,
        name = trader.name,
        freedom = trader.freedom_level,
        perspective = trader.fundamental_perspective,
        paper_account = trader.default_paper_account_id.as_deref().unwrap_or("none"),
        data_sources = if data_sources.is_empty() {
            "none"
        } else {
            &data_sources
        },
        channel_context = channel_context,
        symbols = symbols
    );

    OpenAiChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![
            OpenAiMessage {
                role: "system".to_string(),
                content: "Return only JSON. Never claim live trading or executed trades."
                    .to_string(),
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

fn proposal_request_from_ai(
    trader: &EngineRunnableTrader,
    proposal: TraderAiPortfolioProposal,
    replacement_reason: Option<String>,
    current_prices: &HashMap<String, f64>,
) -> CreateTraderPortfolioProposalRequest {
    let actions = if proposal.actions.is_empty() {
        vec![CreateTraderPortfolioProposalActionRequest {
            symbol: None,
            action_type: "no_action".to_string(),
            side: None,
            quantity: None,
            order_type: None,
            rationale: "No action was provided by the model.".to_string(),
            confidence: proposal.confidence,
            risk_decision: Some("not_applicable".to_string()),
            entry_price: None,
            exit_price: None,
            limit_price: None,
            stop_price: None,
            expected_duration_seconds: proposal.expected_duration_seconds,
            enact_by: None,
            market_price_at_creation: None,
        }]
    } else {
        proposal
            .actions
            .into_iter()
            .map(|action| {
                let risk_decision =
                    risk_decision_for_action(trader, &action.action_type, action.quantity);
                let normalized_symbol = action
                    .symbol
                    .as_ref()
                    .map(|symbol| symbol.trim().to_ascii_uppercase());
                CreateTraderPortfolioProposalActionRequest {
                    symbol: normalized_symbol.clone(),
                    action_type: action.action_type,
                    side: action.side,
                    quantity: action.quantity,
                    order_type: action.order_type,
                    entry_price: action.entry_price,
                    exit_price: action.exit_price,
                    limit_price: action.limit_price,
                    stop_price: action.stop_price,
                    expected_duration_seconds: action
                        .expected_duration_seconds
                        .or(proposal.expected_duration_seconds),
                    enact_by: action.enact_by,
                    market_price_at_creation: action.market_price_at_creation.or_else(|| {
                        normalized_symbol
                            .as_ref()
                            .and_then(|symbol| current_prices.get(symbol).copied())
                    }),
                    rationale: action.rationale,
                    confidence: action.confidence,
                    risk_decision: Some(risk_decision),
                }
            })
            .collect()
    };
    CreateTraderPortfolioProposalRequest {
        trader_id: trader.id.clone(),
        paper_account_id: trader.default_paper_account_id.clone(),
        title: non_empty(proposal.title, "Trader portfolio proposal"),
        summary: non_empty(proposal.summary, "Latest trader proposal."),
        thesis: non_empty(proposal.thesis, "No thesis supplied."),
        confidence: proposal.confidence,
        expected_duration_seconds: proposal.expected_duration_seconds,
        proposed_actions: actions,
        source_snapshot_json: serde_json::to_string(&serde_json::json!({
            "data_sources": trader.data_sources,
            "info_sources": trader.info_sources,
        }))
        .ok(),
        risk_snapshot_json: serde_json::to_string(&serde_json::json!({
            "risk_precheck": "v1 conservative proposal pre-check; no orders submitted",
            "paper_account_id": trader.default_paper_account_id,
        }))
        .ok(),
        market_snapshot_json: serde_json::to_string(&serde_json::json!({
            "tracked_symbols": trader.tracked_symbols,
        }))
        .ok(),
        market_basis_json: proposal
            .market_basis
            .map(|value| value.to_string())
            .or_else(|| {
                serde_json::to_string(&serde_json::json!({
                    "generated_at": Utc::now().to_rfc3339(),
                    "symbols": current_prices.iter().map(|(symbol, price)| {
                        serde_json::json!({"symbol": symbol, "last_price": price})
                    }).collect::<Vec<_>>()
                }))
                .ok()
            }),
        invalidation_conditions_json: proposal
            .invalidation_conditions
            .map(|value| value.to_string()),
        change_thresholds_json: proposal.change_thresholds.map(|value| value.to_string()),
        replacement_reason,
    }
}

fn no_symbol_proposal_request(
    trader: &EngineRunnableTrader,
) -> CreateTraderPortfolioProposalRequest {
    CreateTraderPortfolioProposalRequest {
        trader_id: trader.id.clone(),
        paper_account_id: trader.default_paper_account_id.clone(),
        title: "No active tracked symbols".to_string(),
        summary: "No tracked symbols are active yet.".to_string(),
        thesis: "The trader needs an active/watchlist symbol universe before proposing portfolio actions.".to_string(),
        confidence: Some(1.0),
        expected_duration_seconds: Some(300),
        proposed_actions: vec![CreateTraderPortfolioProposalActionRequest {
            symbol: None,
            action_type: "no_action".to_string(),
            side: None,
            quantity: None,
            order_type: None,
            entry_price: None,
            exit_price: None,
            limit_price: None,
            stop_price: None,
            expected_duration_seconds: Some(300),
            enact_by: None,
            market_price_at_creation: None,
            rationale: "No tracked symbols are active yet.".to_string(),
            confidence: Some(1.0),
            risk_decision: Some("not_applicable".to_string()),
        }],
        source_snapshot_json: None,
        risk_snapshot_json: Some(r#"{"risk_precheck":"not_applicable"}"#.to_string()),
        market_snapshot_json: Some(r#"{"tracked_symbols":[]}"#.to_string()),
        market_basis_json: Some(r#"{"symbols":[]}"#.to_string()),
        invalidation_conditions_json: Some(
            r#"["Add active or watching symbols to create a tradable plan."]"#.to_string(),
        ),
        change_thresholds_json: Some(
            r#"{"material_price_move_percent":3.0,"material_news_change":true,"risk_limit_change":true}"#
                .to_string(),
        ),
        replacement_reason: None,
    }
}

fn fallback_proposal_request(
    trader: &EngineRunnableTrader,
    error: &str,
) -> CreateTraderPortfolioProposalRequest {
    CreateTraderPortfolioProposalRequest {
        trader_id: trader.id.clone(),
        paper_account_id: trader.default_paper_account_id.clone(),
        title: "Hold pending proposal generation".to_string(),
        summary: "Hold current posture because automated proposal generation failed.".to_string(),
        thesis: format!("The engine could not generate a model-backed proposal: {error}"),
        confidence: Some(0.0),
        expected_duration_seconds: Some(300),
        proposed_actions: vec![CreateTraderPortfolioProposalActionRequest {
            symbol: None,
            action_type: "no_action".to_string(),
            side: None,
            quantity: None,
            order_type: None,
            entry_price: None,
            exit_price: None,
            limit_price: None,
            stop_price: None,
            expected_duration_seconds: Some(300),
            enact_by: None,
            market_price_at_creation: None,
            rationale: "No action due to proposal generation failure.".to_string(),
            confidence: Some(0.0),
            risk_decision: Some("not_applicable".to_string()),
        }],
        source_snapshot_json: None,
        risk_snapshot_json: Some(r#"{"risk_precheck":"not_applicable"}"#.to_string()),
        market_snapshot_json: serde_json::to_string(&serde_json::json!({
            "tracked_symbols": trader.tracked_symbols,
        }))
        .ok(),
        market_basis_json: None,
        invalidation_conditions_json: Some(
            r#"["Generate a fresh proposal after model availability returns."]"#.to_string(),
        ),
        change_thresholds_json: Some(
            r#"{"material_price_move_percent":3.0,"material_news_change":true,"risk_limit_change":true}"#
                .to_string(),
        ),
        replacement_reason: Some(error.to_string()),
    }
}

fn risk_decision_for_action(
    trader: &EngineRunnableTrader,
    action_type: &str,
    quantity: Option<f64>,
) -> String {
    match action_type {
        "buy" | "sell" | "reduce" | "increase" => {
            if trader.default_paper_account_id.is_none() {
                "blocked:no_paper_account".to_string()
            } else if quantity.unwrap_or(0.0) <= 0.0 {
                "blocked:missing_or_invalid_quantity".to_string()
            } else {
                "proposal_only:paper_risk_checked_before_execution".to_string()
            }
        }
        _ => "not_applicable".to_string(),
    }
}

fn non_empty(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn build_openai_request(
    trader: &EngineRunnableTrader,
    channel_insight: Option<&channels::TraderChannelInsight>,
) -> OpenAiChatRequest {
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
    let tracked_symbols = trader
        .tracked_symbols
        .iter()
        .filter(|symbol| symbol.status == "active" || symbol.status == "watching")
        .map(|symbol| {
            format!(
                "{}:{}:{} fit={:?} thesis={}",
                symbol.symbol,
                symbol.asset_type,
                symbol.status,
                symbol.fit_score,
                symbol.thesis.as_deref().unwrap_or("none")
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let paper_account = trader.default_paper_account_id.as_deref().unwrap_or("none");
    let channel_context = channel_insight
        .map(|insight| insight.summary.as_str())
        .unwrap_or("Channels disabled or no channel context loaded.");
    let prompt = format!(
        "You are an autonomous paper-trading assistant named {name}. Perspective: {perspective}. Freedom level: {freedom}. Paper account: {paper_account}. Info source placeholders: {sources}. Assigned data sources: {assigned_sources}. Tracked symbol universe: {tracked_symbols}. Channel context: {channel_context}. At the beginning of this evaluation, use channel context to see whether your previous questions were answered and whether there is new user, MD, or trader information that changes your reasoning. Evaluate only active/watching tracked symbols. Return strict JSON with action hold, propose_trade, or trade; optional symbol, side, quantity; reason; confidence. Use conservative risk-aware behavior. Channel discussion cannot execute trades or bypass risk controls.",
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
        tracked_symbols = if tracked_symbols.is_empty() {
            "none"
        } else {
            &tracked_symbols
        },
        channel_context = channel_context,
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
    if let Some(symbol) = decision.symbol.as_deref() {
        let symbol = symbol.trim().to_ascii_uppercase();
        let allowed = trader.tracked_symbols.iter().any(|tracked| {
            tracked.symbol == symbol && (tracked.status == "active" || tracked.status == "watching")
        });
        if !allowed {
            client
                .create_event(
                    &trader.id,
                    &CreateTraderEventRequest {
                        event_type: "trade_blocked_outside_symbol_universe".to_string(),
                        message: format!(
                            "Blocked trader decision for {symbol}; symbol is not active/watching in this trader's universe"
                        ),
                        payload: serde_json::to_string(&serde_json::json!({
                            "symbol": symbol,
                            "action": decision.action,
                            "reason": decision.reason,
                        }))
                        .ok(),
                    },
                )
                .await?;
            return Ok(());
        }
    }

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
