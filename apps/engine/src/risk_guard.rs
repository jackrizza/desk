use chrono::{DateTime, Utc};

use crate::types::{
    EngineRunnableStrategy, PaperAccountSummaryResponse, ProposedPaperOrder, RiskDecision,
    StrategyRuntimeState, StrategySignal,
};

pub fn evaluate_order_risk(
    strategy: &EngineRunnableStrategy,
    proposed_order: &ProposedPaperOrder,
    account_summary: &PaperAccountSummaryResponse,
    runtime_state: &StrategyRuntimeState,
    recent_signals: &[StrategySignal],
) -> RiskDecision {
    let risk = &strategy.risk_config;

    if risk.kill_switch_enabled {
        return reject("kill switch enabled");
    }

    if !risk.is_trading_enabled {
        return reject("trading disabled by risk config");
    }

    if let Some(symbols) = &risk.allowlist_symbols {
        if !symbols
            .iter()
            .any(|symbol| symbol == &proposed_order.symbol)
        {
            return reject("symbol not in allowlist");
        }
    }

    if let Some(symbols) = &risk.blocklist_symbols {
        if symbols
            .iter()
            .any(|symbol| symbol == &proposed_order.symbol)
        {
            return reject("symbol is blocklisted");
        }
    }

    if let Some(limit) = risk.max_quantity_per_trade {
        if proposed_order.quantity > limit {
            return reject("proposed quantity exceeds max quantity per trade");
        }
    }

    if let Some(limit) = risk.max_dollars_per_trade {
        if proposed_order.estimated_notional > limit {
            return reject("proposed order exceeds max dollars per trade");
        }
    }

    if let Some(limit) = risk.max_position_value_per_symbol {
        let current_symbol_value = account_summary
            .positions
            .iter()
            .filter(|position| position.symbol == proposed_order.symbol && position.quantity > 0.0)
            .map(|position| position.quantity * proposed_order.estimated_price)
            .sum::<f64>();
        let resulting_value = match proposed_order.side.as_str() {
            "buy" => current_symbol_value + proposed_order.estimated_notional,
            "sell" => current_symbol_value - proposed_order.estimated_notional,
            _ => current_symbol_value,
        };
        if resulting_value > limit {
            return reject("resulting symbol exposure exceeds max position value per symbol");
        }
    }

    if let Some(limit) = risk.max_total_exposure {
        let current_exposure = account_summary
            .positions
            .iter()
            .filter(|position| position.quantity > 0.0)
            .map(|position| position.quantity * proposed_order.estimated_price)
            .sum::<f64>();
        let resulting_exposure = match proposed_order.side.as_str() {
            "buy" => current_exposure + proposed_order.estimated_notional,
            "sell" => (current_exposure - proposed_order.estimated_notional).max(0.0),
            _ => current_exposure,
        };
        if resulting_exposure > limit {
            return reject("resulting account exposure exceeds max total exposure");
        }
    }

    if let Some(limit) = risk.max_open_positions {
        let open_positions = account_summary
            .positions
            .iter()
            .filter(|position| position.quantity > 0.0)
            .count() as i64;
        let adds_new_position = proposed_order.side == "buy"
            && !account_summary.positions.iter().any(|position| {
                position.symbol == proposed_order.symbol && position.quantity > 0.0
            });
        if adds_new_position && open_positions >= limit {
            return reject("max open positions reached");
        }
    }

    if let Some(limit) = risk.max_daily_trades {
        let daily_trade_count = recent_signals
            .iter()
            .filter(|signal| signal.status == "submitted" || signal.status == "filled")
            .filter(|signal| is_today(&signal.created_at))
            .count() as i64;
        if daily_trade_count >= limit {
            return reject("max daily trades reached");
        }
    }

    if let Some(limit) = risk.max_daily_loss {
        let realized_loss = account_summary
            .positions
            .iter()
            .map(|position| position.realized_pnl.min(0.0))
            .sum::<f64>()
            .abs();
        if realized_loss >= limit {
            return reject("max daily loss reached");
        }
    }

    if let Some(cooldown_until) = &runtime_state.cooldown_until {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(cooldown_until) {
            if parsed.with_timezone(&Utc) > Utc::now() {
                return reject("cooldown active");
            }
        }
    }

    RiskDecision {
        allowed: true,
        reason: "accepted".to_string(),
        adjusted_quantity: None,
    }
}

fn reject(reason: &str) -> RiskDecision {
    RiskDecision {
        allowed: false,
        reason: reason.to_string(),
        adjusted_quantity: None,
    }
}

fn is_today(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc).date_naive() == Utc::now().date_naive())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::evaluate_order_risk;
    use crate::types::{
        EngineRunnableStrategy, PaperAccountSummaryResponse, ProposedPaperOrder,
        StrategyDefinition, StrategyPositionSize, StrategyRisk, StrategyRiskConfig,
        StrategyRuntimeState, StrategySignal,
    };
    use models::paper::{PaperAccount, PaperPosition};

    fn strategy() -> EngineRunnableStrategy {
        EngineRunnableStrategy {
            strategy_id: "strategy-1".to_string(),
            name: "Test".to_string(),
            trading_mode: "paper".to_string(),
            paper_account_id: "account-1".to_string(),
            symbol_universe: vec!["AAPL".to_string()],
            timeframe: "1d".to_string(),
            strategy_definition: StrategyDefinition::default(),
            risk: StrategyRisk {
                position_size: StrategyPositionSize {
                    r#type: "fixed_quantity".to_string(),
                    quantity: Some(1.0),
                    percent: None,
                },
                max_position_per_symbol: Some(1),
                cooldown_seconds: Some(300),
            },
            risk_config: StrategyRiskConfig {
                strategy_id: "strategy-1".to_string(),
                max_dollars_per_trade: Some(500.0),
                max_quantity_per_trade: Some(5.0),
                max_position_value_per_symbol: Some(1000.0),
                max_total_exposure: Some(5000.0),
                max_open_positions: Some(3),
                max_daily_trades: Some(5),
                max_daily_loss: Some(250.0),
                cooldown_seconds: 300,
                allowlist_symbols: Some(vec!["AAPL".to_string()]),
                blocklist_symbols: None,
                is_trading_enabled: true,
                kill_switch_enabled: false,
                created_at: None,
                updated_at: None,
            },
        }
    }

    fn account_summary() -> PaperAccountSummaryResponse {
        PaperAccountSummaryResponse {
            account: PaperAccount {
                id: "account-1".to_string(),
                name: "Paper".to_string(),
                starting_cash: 10000.0,
                cash_balance: 9500.0,
                currency: "USD".to_string(),
                is_active: true,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
            positions: vec![PaperPosition {
                id: "position-1".to_string(),
                account_id: "account-1".to_string(),
                symbol: "MSFT".to_string(),
                quantity: 1.0,
                average_price: 100.0,
                realized_pnl: 0.0,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            }],
            open_orders: vec![],
            recent_fills: vec![],
            equity_estimate: 9600.0,
        }
    }

    fn runtime_state() -> StrategyRuntimeState {
        StrategyRuntimeState {
            id: "runtime".to_string(),
            strategy_id: "strategy-1".to_string(),
            paper_account_id: "account-1".to_string(),
            symbol: "AAPL".to_string(),
            last_evaluated_at: None,
            last_signal: None,
            last_signal_at: None,
            last_order_id: None,
            position_state: "flat".to_string(),
            cooldown_until: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn proposed_order() -> ProposedPaperOrder {
        ProposedPaperOrder {
            strategy_id: "strategy-1".to_string(),
            signal_id: "signal-1".to_string(),
            account_id: "account-1".to_string(),
            symbol: "AAPL".to_string(),
            side: "buy".to_string(),
            order_type: "market".to_string(),
            quantity: 2.0,
            estimated_price: 100.0,
            estimated_notional: 200.0,
        }
    }

    #[test]
    fn rejects_over_max_dollars_per_trade() {
        let mut order = proposed_order();
        order.estimated_notional = 600.0;
        let decision = evaluate_order_risk(
            &strategy(),
            &order,
            &account_summary(),
            &runtime_state(),
            &[],
        );
        assert!(!decision.allowed);
    }

    #[test]
    fn rejects_blocklisted_symbol() {
        let mut strategy = strategy();
        strategy.risk_config.blocklist_symbols = Some(vec!["AAPL".to_string()]);
        let decision = evaluate_order_risk(
            &strategy,
            &proposed_order(),
            &account_summary(),
            &runtime_state(),
            &[],
        );
        assert!(!decision.allowed);
    }

    #[test]
    fn rejects_when_kill_switch_enabled() {
        let mut strategy = strategy();
        strategy.risk_config.kill_switch_enabled = true;
        let decision = evaluate_order_risk(
            &strategy,
            &proposed_order(),
            &account_summary(),
            &runtime_state(),
            &[],
        );
        assert!(!decision.allowed);
    }
}
