use crate::indicators::{ema, rsi, sma};
use crate::types::{StrategyCondition, StrategyConditionGroup, StrategyDefinition};

#[derive(Clone, Debug, PartialEq)]
pub enum SignalDecision {
    EnterLong { reason: String },
    ExitLong { reason: String },
    Hold { reason: String },
    NoAction { reason: String },
}

#[derive(Clone, Debug, Default)]
pub struct EvaluationContext {
    pub closes: Vec<f64>,
    pub position_state: String,
}

pub fn evaluate_strategy(
    strategy: &StrategyDefinition,
    context: &EvaluationContext,
) -> SignalDecision {
    let is_long = context.position_state == "long";
    let entry = evaluate_group(&strategy.entry, &context.closes);
    let exit = evaluate_group(&strategy.exit, &context.closes);

    if !is_long && entry {
        return SignalDecision::EnterLong {
            reason: "entry conditions met".to_string(),
        };
    }

    if is_long && exit {
        return SignalDecision::ExitLong {
            reason: "exit conditions met".to_string(),
        };
    }

    if is_long {
        SignalDecision::Hold {
            reason: "holding current long position".to_string(),
        }
    } else {
        SignalDecision::NoAction {
            reason: "no trade conditions met".to_string(),
        }
    }
}

fn evaluate_group(group: &StrategyConditionGroup, closes: &[f64]) -> bool {
    let all_pass = group.all.as_ref().map(|conditions| {
        conditions
            .iter()
            .all(|condition| evaluate_condition(condition, closes))
    });
    let any_pass = group.any.as_ref().map(|conditions| {
        conditions
            .iter()
            .any(|condition| evaluate_condition(condition, closes))
    });

    match (all_pass, any_pass) {
        (Some(all), Some(any)) => all && any,
        (Some(all), None) => all,
        (None, Some(any)) => any,
        (None, None) => false,
    }
}

fn evaluate_condition(condition: &StrategyCondition, closes: &[f64]) -> bool {
    match condition.operator.as_str() {
        "greater_than" => compare_latest(condition, closes, |left, right| left > right),
        "less_than" => compare_latest(condition, closes, |left, right| left < right),
        "crosses_above" => {
            compare_cross(condition, closes, |prev_left, prev_right, left, right| {
                prev_left <= prev_right && left > right
            })
        }
        "crosses_below" => {
            compare_cross(condition, closes, |prev_left, prev_right, left, right| {
                prev_left >= prev_right && left < right
            })
        }
        _ => false,
    }
}

fn compare_latest(
    condition: &StrategyCondition,
    closes: &[f64],
    predicate: impl Fn(f64, f64) -> bool,
) -> bool {
    let Some(left) = indicator_value(closes, &condition.indicator, condition.period, 0) else {
        return false;
    };
    let Some(right) = comparison_value(condition, closes, 0) else {
        return false;
    };

    predicate(left, right)
}

fn compare_cross(
    condition: &StrategyCondition,
    closes: &[f64],
    predicate: impl Fn(f64, f64, f64, f64) -> bool,
) -> bool {
    let Some(prev_left) = indicator_value(closes, &condition.indicator, condition.period, 1) else {
        return false;
    };
    let Some(left) = indicator_value(closes, &condition.indicator, condition.period, 0) else {
        return false;
    };
    let Some(prev_right) = comparison_value(condition, closes, 1) else {
        return false;
    };
    let Some(right) = comparison_value(condition, closes, 0) else {
        return false;
    };

    predicate(prev_left, prev_right, left, right)
}

fn comparison_value(condition: &StrategyCondition, closes: &[f64], offset: usize) -> Option<f64> {
    if let Some(value) = condition.value {
        return Some(value);
    }

    indicator_value(
        closes,
        condition.compare_indicator.as_deref().unwrap_or("close"),
        condition.compare_period,
        offset,
    )
}

fn indicator_value(
    closes: &[f64],
    indicator: &str,
    period: Option<i32>,
    offset: usize,
) -> Option<f64> {
    let series = closes
        .get(..closes.len().checked_sub(offset)?)
        .unwrap_or_default();
    let period = period.unwrap_or(1).max(1) as usize;

    match indicator {
        "close" | "latest_close" => series.last().copied(),
        "sma" => sma(series, period),
        "ema" => ema(series, period),
        "rsi" => rsi(series, period),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{EvaluationContext, SignalDecision, evaluate_strategy};
    use crate::types::{
        StrategyCondition, StrategyConditionGroup, StrategyDefinition, StrategyPositionSize,
        StrategyRisk,
    };

    fn sample_strategy() -> StrategyDefinition {
        StrategyDefinition {
            version: "1".to_string(),
            entry: StrategyConditionGroup {
                all: Some(vec![StrategyCondition {
                    indicator: "sma".to_string(),
                    period: Some(2),
                    operator: "crosses_above".to_string(),
                    value: None,
                    compare_indicator: Some("sma".to_string()),
                    compare_period: Some(3),
                }]),
                any: None,
            },
            exit: StrategyConditionGroup {
                all: Some(vec![StrategyCondition {
                    indicator: "rsi".to_string(),
                    period: Some(2),
                    operator: "greater_than".to_string(),
                    value: Some(70.0),
                    compare_indicator: None,
                    compare_period: None,
                }]),
                any: None,
            },
            risk: StrategyRisk {
                position_size: StrategyPositionSize {
                    r#type: "fixed_quantity".to_string(),
                    quantity: Some(1.0),
                    percent: None,
                },
                max_position_per_symbol: Some(1),
                cooldown_seconds: Some(3600),
            },
        }
    }

    #[test]
    fn flat_and_entry_true_enters_long() {
        let decision = evaluate_strategy(
            &sample_strategy(),
            &EvaluationContext {
                closes: vec![10.0, 9.0, 11.0, 15.0],
                position_state: "flat".to_string(),
            },
        );

        assert!(matches!(decision, SignalDecision::EnterLong { .. }));
    }

    #[test]
    fn long_and_exit_true_exits_long() {
        let decision = evaluate_strategy(
            &sample_strategy(),
            &EvaluationContext {
                closes: vec![10.0, 11.0, 12.0, 18.0],
                position_state: "long".to_string(),
            },
        );

        assert!(matches!(decision, SignalDecision::ExitLong { .. }));
    }
}
