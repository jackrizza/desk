import type { PaperAccountSummaryResponse, StrategyRiskConfig } from "./api";

function compareBySymbolAsc(left: { symbol: string }, right: { symbol: string }) {
  return left.symbol.localeCompare(right.symbol);
}

function compareByCreatedAtDesc(
  left: { created_at: string; id: string },
  right: { created_at: string; id: string },
) {
  const createdAtCompare = right.created_at.localeCompare(left.created_at);
  if (createdAtCompare !== 0) {
    return createdAtCompare;
  }

  return right.id.localeCompare(left.id);
}

function normalizePaperSummary(summary: PaperAccountSummaryResponse) {
  return {
    account: summary.account,
    positions: [...summary.positions].sort(compareBySymbolAsc),
    open_orders: [...summary.open_orders].sort(compareByCreatedAtDesc),
    recent_fills: [...summary.recent_fills].sort(compareByCreatedAtDesc),
    equity_estimate: summary.equity_estimate,
    total_cost_basis: summary.total_cost_basis ?? null,
    total_market_value: summary.total_market_value ?? null,
    total_unrealized_gain: summary.total_unrealized_gain ?? null,
    total_unrealized_gain_percent: summary.total_unrealized_gain_percent ?? null,
  };
}

export function paperSummariesEqual(
  left: PaperAccountSummaryResponse | null | undefined,
  right: PaperAccountSummaryResponse | null | undefined,
) {
  if (!left && !right) {
    return true;
  }

  if (!left || !right) {
    return false;
  }

  return JSON.stringify(normalizePaperSummary(left)) === JSON.stringify(normalizePaperSummary(right));
}

function normalizeRiskConfig(config: StrategyRiskConfig) {
  return {
    strategy_id: config.strategy_id,
    max_dollars_per_trade: config.max_dollars_per_trade ?? null,
    max_quantity_per_trade: config.max_quantity_per_trade ?? null,
    max_position_value_per_symbol: config.max_position_value_per_symbol ?? null,
    max_total_exposure: config.max_total_exposure ?? null,
    max_open_positions: config.max_open_positions ?? null,
    max_daily_trades: config.max_daily_trades ?? null,
    max_daily_loss: config.max_daily_loss ?? null,
    cooldown_seconds: config.cooldown_seconds,
    allowlist_symbols: [...(config.allowlist_symbols ?? [])].sort((left, right) => left.localeCompare(right)),
    blocklist_symbols: [...(config.blocklist_symbols ?? [])].sort((left, right) => left.localeCompare(right)),
    is_trading_enabled: config.is_trading_enabled,
    kill_switch_enabled: config.kill_switch_enabled,
  };
}

export function strategyRiskConfigsEqual(
  left: StrategyRiskConfig | null | undefined,
  right: StrategyRiskConfig | null | undefined,
) {
  if (!left && !right) {
    return true;
  }

  if (!left || !right) {
    return false;
  }

  return JSON.stringify(normalizeRiskConfig(left)) === JSON.stringify(normalizeRiskConfig(right));
}
