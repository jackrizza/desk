use crate::types::StrategyDefinition;

pub fn fixed_quantity(strategy: &StrategyDefinition) -> f64 {
    match strategy.risk.position_size.r#type.as_str() {
        "fixed_quantity" => strategy.risk.position_size.quantity.unwrap_or(1.0).max(0.0),
        _ => 1.0,
    }
}
