use std::collections::HashMap;

use chrono::{DateTime, Utc};
use models::trader::{EngineRunnableTrader, TraderPortfolioProposalDetail};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct PlanMonitorDecision {
    pub should_replace: bool,
    pub reason: String,
    pub material_changes: Vec<String>,
}

pub fn evaluate_active_plan(
    trader: &EngineRunnableTrader,
    active_plan: &TraderPortfolioProposalDetail,
    current_prices: &HashMap<String, f64>,
) -> PlanMonitorDecision {
    let mut changes = Vec::new();
    let price_threshold = material_price_threshold(active_plan);

    if let Some(active_until) = &active_plan.proposal.active_until {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(active_until) {
            if parsed.with_timezone(&Utc) <= Utc::now() {
                changes.push("active plan duration expired".to_string());
            }
        }
    }

    for action in &active_plan.actions {
        if let Some(symbol) = &action.symbol {
            let still_tracked = trader.tracked_symbols.iter().any(|tracked| {
                tracked.symbol == *symbol
                    && (tracked.status == "active"
                        || tracked.status == "watching"
                        || tracked.status == "candidate")
            });
            if !still_tracked {
                changes.push(format!(
                    "{symbol} is no longer in the active/watchlist universe"
                ));
            }

            if let (Some(current), Some(basis)) = (
                current_prices.get(symbol),
                action
                    .market_price_at_creation
                    .or_else(|| basis_price(active_plan, symbol)),
            ) {
                let change = ((*current - basis) / basis).abs() * 100.0;
                if change >= price_threshold {
                    changes.push(format!(
                        "{symbol} moved {change:.2}% from active plan basis"
                    ));
                }
            }
            if let (Some(current), Some(stop)) = (current_prices.get(symbol), action.stop_price) {
                if *current <= stop {
                    changes.push(format!("{symbol} reached or breached stop price {stop}"));
                }
            }
            if let (Some(current), Some(exit)) = (current_prices.get(symbol), action.exit_price) {
                if *current >= exit {
                    changes.push(format!("{symbol} reached or exceeded exit price {exit}"));
                }
            }
        }
    }

    if changes.is_empty() {
        PlanMonitorDecision {
            should_replace: false,
            reason: "Active plan remains valid; no deterministic material change detected."
                .to_string(),
            material_changes: Vec::new(),
        }
    } else {
        PlanMonitorDecision {
            should_replace: true,
            reason: changes.join("; "),
            material_changes: changes,
        }
    }
}

fn material_price_threshold(active_plan: &TraderPortfolioProposalDetail) -> f64 {
    active_plan
        .proposal
        .change_thresholds_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<Value>(json).ok())
        .and_then(|value| {
            value
                .get("material_price_move_percent")
                .and_then(Value::as_f64)
        })
        .unwrap_or(3.0)
}

fn basis_price(active_plan: &TraderPortfolioProposalDetail, symbol: &str) -> Option<f64> {
    active_plan
        .proposal
        .market_basis_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<Value>(json).ok())
        .and_then(|value| value.get("symbols").and_then(Value::as_array).cloned())
        .and_then(|symbols| {
            symbols.into_iter().find_map(|entry| {
                let entry_symbol = entry.get("symbol").and_then(Value::as_str)?;
                if entry_symbol.eq_ignore_ascii_case(symbol) {
                    entry.get("last_price").and_then(Value::as_f64)
                } else {
                    None
                }
            })
        })
}
