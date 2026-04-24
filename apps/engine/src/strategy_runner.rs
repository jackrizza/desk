use chrono::{Duration, Utc};

use crate::{
    client::OpenApiClient,
    config::EngineConfig,
    risk::fixed_quantity,
    risk_guard::evaluate_order_risk,
    strategy_eval::{EvaluationContext, SignalDecision, evaluate_strategy},
    types::{
        CreateStrategySignalRequest, EngineRunnableStrategy, ProposedPaperOrder,
        SubmitPaperOrderRequest, UpdateStrategySignalStatusRequest,
        UpsertStrategyRuntimeStateRequest,
    },
};

pub async fn run_strategy(
    config: &EngineConfig,
    client: &OpenApiClient,
    strategy: &EngineRunnableStrategy,
    symbol: &str,
    closes: Vec<f64>,
) -> Result<(), String> {
    let runtime_state = client
        .fetch_strategy_runtime_state(&strategy.strategy_id)
        .await
        .map_err(|err| format!("failed to fetch runtime state: {err}"))?
        .into_iter()
        .find(|state| state.paper_account_id == strategy.paper_account_id && state.symbol == symbol)
        .unwrap_or_else(|| crate::types::StrategyRuntimeState {
            id: String::new(),
            strategy_id: strategy.strategy_id.clone(),
            paper_account_id: strategy.paper_account_id.clone(),
            symbol: symbol.to_string(),
            last_evaluated_at: None,
            last_signal: None,
            last_signal_at: None,
            last_order_id: None,
            position_state: "flat".to_string(),
            cooldown_until: None,
            created_at: String::new(),
            updated_at: String::new(),
        });

    if let Some(cooldown_until) = &runtime_state.cooldown_until {
        if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(cooldown_until) {
            if parsed.with_timezone(&Utc) > Utc::now() {
                client
                    .upsert_strategy_runtime_state(&UpsertStrategyRuntimeStateRequest {
                        strategy_id: strategy.strategy_id.clone(),
                        paper_account_id: strategy.paper_account_id.clone(),
                        symbol: symbol.to_string(),
                        last_evaluated_at: Some(Utc::now().to_rfc3339()),
                        last_signal: runtime_state.last_signal.clone(),
                        last_signal_at: runtime_state.last_signal_at.clone(),
                        last_order_id: runtime_state.last_order_id.clone(),
                        position_state: runtime_state.position_state.clone(),
                        cooldown_until: runtime_state.cooldown_until.clone(),
                    })
                    .await
                    .map_err(|err| format!("failed to refresh cooldown runtime state: {err}"))?;
                return Ok(());
            }
        }
    }

    let decision = evaluate_strategy(
        &strategy.strategy_definition,
        &EvaluationContext {
            closes: closes.clone(),
            position_state: runtime_state.position_state.clone(),
        },
    );

    let (signal_type, reason) = match decision {
        SignalDecision::EnterLong { reason } => ("enter_long".to_string(), reason),
        SignalDecision::ExitLong { reason } => ("exit_long".to_string(), reason),
        SignalDecision::Hold { reason } => ("hold".to_string(), reason),
        SignalDecision::NoAction { reason } => ("no_action".to_string(), reason),
    };

    let latest_price = closes.last().copied();
    let Some(latest_price) = latest_price else {
        return Err("missing latest price for strategy evaluation".to_string());
    };

    let mut next_position_state = runtime_state.position_state.clone();
    let mut last_order_id = runtime_state.last_order_id.clone();
    let now = Utc::now().to_rfc3339();
    let account_summary = client
        .get_paper_account_summary(&strategy.paper_account_id)
        .await
        .map_err(|err| format!("failed to load paper account summary: {err}"))?;
    let recent_signals = client
        .fetch_strategy_signals(&strategy.strategy_id)
        .await
        .map_err(|err| format!("failed to load recent strategy signals: {err}"))?;

    if signal_type == "enter_long" && runtime_state.position_state == "flat" {
        let proposed_order = ProposedPaperOrder {
            strategy_id: strategy.strategy_id.clone(),
            signal_id: String::new(),
            account_id: strategy.paper_account_id.clone(),
            symbol: symbol.to_string(),
            side: "buy".to_string(),
            order_type: "market".to_string(),
            quantity: fixed_quantity(&strategy.strategy_definition),
            estimated_price: latest_price,
            estimated_notional: fixed_quantity(&strategy.strategy_definition) * latest_price,
        };

        let risk_decision = evaluate_order_risk(
            strategy,
            &proposed_order,
            &account_summary,
            &runtime_state,
            &recent_signals,
        );

        let signal = client
            .create_strategy_signal(&CreateStrategySignalRequest {
                strategy_id: strategy.strategy_id.clone(),
                paper_account_id: strategy.paper_account_id.clone(),
                symbol: symbol.to_string(),
                signal_type: signal_type.clone(),
                confidence: None,
                reason: reason.clone(),
                market_price: Some(latest_price),
                source: Some("engine".to_string()),
                status: Some(if risk_decision.allowed {
                    "created".to_string()
                } else {
                    "blocked_by_risk".to_string()
                }),
                risk_decision: Some(if risk_decision.allowed {
                    "accepted".to_string()
                } else {
                    "rejected".to_string()
                }),
                risk_reason: Some(risk_decision.reason.clone()),
                order_id: None,
            })
            .await
            .map_err(|err| format!("failed to create strategy signal: {err}"))?;

        if !risk_decision.allowed {
            client
                .report_engine_event(&crate::types::EngineEventRequest {
                    engine_name: config.engine_name.clone(),
                    event_type: "strategy_risk_blocked".to_string(),
                    symbol: Some(symbol.to_string()),
                    message: format!(
                        "strategy {} blocked enter on {}: {}",
                        strategy.strategy_id, symbol, risk_decision.reason
                    ),
                    timestamp: now.clone(),
                })
                .await
                .map_err(|err| format!("failed to report risk block event: {err}"))?;
        } else {
            let order = client
                .submit_paper_order(&SubmitPaperOrderRequest {
                    account_id: strategy.paper_account_id.clone(),
                    symbol: symbol.to_string(),
                    side: "buy".to_string(),
                    order_type: "market".to_string(),
                    quantity: proposed_order.quantity,
                    requested_price: None,
                    source: Some("engine".to_string()),
                    trader_id: None,
                    strategy_id: Some(strategy.strategy_id.clone()),
                    signal_id: Some(signal.id.clone()),
                    proposal_id: None,
                })
                .await
                .map_err(|err| format!("failed to submit enter paper order: {err}"))?;
            client
                .update_strategy_signal(
                    &signal.id,
                    &UpdateStrategySignalStatusRequest {
                        status: "filled".to_string(),
                        risk_decision: Some("accepted".to_string()),
                        risk_reason: Some("order submitted".to_string()),
                        order_id: Some(order.order.id.clone()),
                    },
                )
                .await
                .map_err(|err| format!("failed to update submitted signal: {err}"))?;
            next_position_state = "long".to_string();
            last_order_id = Some(order.order.id);
        }
    } else if signal_type == "exit_long" && runtime_state.position_state == "long" {
        let quantity = account_summary
            .positions
            .iter()
            .find(|position| position.symbol == symbol)
            .map(|position| position.quantity)
            .unwrap_or(0.0);
        if quantity > 0.0 {
            let proposed_order = ProposedPaperOrder {
                strategy_id: strategy.strategy_id.clone(),
                signal_id: String::new(),
                account_id: strategy.paper_account_id.clone(),
                symbol: symbol.to_string(),
                side: "sell".to_string(),
                order_type: "market".to_string(),
                quantity,
                estimated_price: latest_price,
                estimated_notional: quantity * latest_price,
            };
            let risk_decision = evaluate_order_risk(
                strategy,
                &proposed_order,
                &account_summary,
                &runtime_state,
                &recent_signals,
            );
            let signal = client
                .create_strategy_signal(&CreateStrategySignalRequest {
                    strategy_id: strategy.strategy_id.clone(),
                    paper_account_id: strategy.paper_account_id.clone(),
                    symbol: symbol.to_string(),
                    signal_type: signal_type.clone(),
                    confidence: None,
                    reason: reason.clone(),
                    market_price: Some(latest_price),
                    source: Some("engine".to_string()),
                    status: Some(if risk_decision.allowed {
                        "created".to_string()
                    } else {
                        "blocked_by_risk".to_string()
                    }),
                    risk_decision: Some(if risk_decision.allowed {
                        "accepted".to_string()
                    } else {
                        "rejected".to_string()
                    }),
                    risk_reason: Some(risk_decision.reason.clone()),
                    order_id: None,
                })
                .await
                .map_err(|err| format!("failed to create exit strategy signal: {err}"))?;

            if !risk_decision.allowed {
                client
                    .report_engine_event(&crate::types::EngineEventRequest {
                        engine_name: config.engine_name.clone(),
                        event_type: "strategy_risk_blocked".to_string(),
                        symbol: Some(symbol.to_string()),
                        message: format!(
                            "strategy {} blocked exit on {}: {}",
                            strategy.strategy_id, symbol, risk_decision.reason
                        ),
                        timestamp: now.clone(),
                    })
                    .await
                    .map_err(|err| format!("failed to report risk block event: {err}"))?;
            } else {
                let order = client
                    .submit_paper_order(&SubmitPaperOrderRequest {
                        account_id: strategy.paper_account_id.clone(),
                        symbol: symbol.to_string(),
                        side: "sell".to_string(),
                        order_type: "market".to_string(),
                        quantity,
                        requested_price: None,
                        source: Some("engine".to_string()),
                        trader_id: None,
                        strategy_id: Some(strategy.strategy_id.clone()),
                        signal_id: Some(signal.id.clone()),
                        proposal_id: None,
                    })
                    .await
                    .map_err(|err| format!("failed to submit exit paper order: {err}"))?;
                client
                    .update_strategy_signal(
                        &signal.id,
                        &UpdateStrategySignalStatusRequest {
                            status: "filled".to_string(),
                            risk_decision: Some("accepted".to_string()),
                            risk_reason: Some("order submitted".to_string()),
                            order_id: Some(order.order.id.clone()),
                        },
                    )
                    .await
                    .map_err(|err| format!("failed to update exit signal: {err}"))?;
                next_position_state = "flat".to_string();
                last_order_id = Some(order.order.id);
            }
        }
    } else {
        let _ = client
            .create_strategy_signal(&CreateStrategySignalRequest {
                strategy_id: strategy.strategy_id.clone(),
                paper_account_id: strategy.paper_account_id.clone(),
                symbol: symbol.to_string(),
                signal_type: signal_type.clone(),
                confidence: None,
                reason: reason.clone(),
                market_price: Some(latest_price),
                source: Some("engine".to_string()),
                status: Some("created".to_string()),
                risk_decision: None,
                risk_reason: None,
                order_id: None,
            })
            .await;
    }

    let cooldown = strategy.risk_config.cooldown_seconds;
    client
        .upsert_strategy_runtime_state(&UpsertStrategyRuntimeStateRequest {
            strategy_id: strategy.strategy_id.clone(),
            paper_account_id: strategy.paper_account_id.clone(),
            symbol: symbol.to_string(),
            last_evaluated_at: Some(now.clone()),
            last_signal: Some(signal_type),
            last_signal_at: Some(now.clone()),
            last_order_id,
            position_state: next_position_state,
            cooldown_until: if cooldown > 0 {
                Some((Utc::now() + Duration::seconds(cooldown)).to_rfc3339())
            } else {
                None
            },
        })
        .await
        .map_err(|err| format!("failed to update strategy runtime state: {err}"))?;

    if config.enable_test_paper_orders {
        let _ = &strategy.name;
    }

    Ok(())
}
