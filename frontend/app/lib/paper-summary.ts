import type { PaperAccountSummaryResponse, PaperPositionSummary } from "./api";

function safeNumber(value: number | null | undefined) {
  return Number.isFinite(value) ? Number(value) : 0;
}

export function getPaperPositionMetrics(position: PaperPositionSummary) {
  const quantity = safeNumber(position.quantity);
  const averagePrice = safeNumber(position.average_price);
  const costBasis = Number.isFinite(position.cost_basis)
    ? position.cost_basis
    : quantity * averagePrice;
  const currentPrice =
    position.current_price != null && Number.isFinite(position.current_price)
      ? position.current_price
      : averagePrice;
  const marketValue =
    position.market_value != null && Number.isFinite(position.market_value)
      ? position.market_value
      : quantity * currentPrice;
  const unrealizedGain = Number.isFinite(position.unrealized_gain)
    ? position.unrealized_gain
    : marketValue - costBasis;
  const unrealizedGainPercent =
    Number.isFinite(position.unrealized_gain_percent)
      ? position.unrealized_gain_percent
      : costBasis !== 0
        ? (unrealizedGain / costBasis) * 100
        : 0;

  return {
    currentPrice,
    marketValue,
    costBasis,
    unrealizedGain,
    unrealizedGainPercent,
  };
}

export function getPaperSummaryMetrics(summary: PaperAccountSummaryResponse) {
  const fallback = summary.positions.reduce(
    (totals, position) => {
      const metrics = getPaperPositionMetrics(position);
      return {
        costBasis: totals.costBasis + metrics.costBasis,
        marketValue: totals.marketValue + metrics.marketValue,
        unrealizedGain: totals.unrealizedGain + metrics.unrealizedGain,
      };
    },
    {
      costBasis: 0,
      marketValue: 0,
      unrealizedGain: 0,
    },
  );

  const costBasis = Number.isFinite(summary.total_cost_basis)
    ? Number(summary.total_cost_basis)
    : fallback.costBasis;
  const marketValue = Number.isFinite(summary.total_market_value)
    ? Number(summary.total_market_value)
    : fallback.marketValue;
  const unrealizedGain = Number.isFinite(summary.total_unrealized_gain)
    ? Number(summary.total_unrealized_gain)
    : fallback.unrealizedGain;
  const unrealizedGainPercent = Number.isFinite(summary.total_unrealized_gain_percent)
    ? Number(summary.total_unrealized_gain_percent)
    : costBasis !== 0
      ? (unrealizedGain / costBasis) * 100
      : 0;

  return {
    costBasis,
    marketValue,
    unrealizedGain,
    unrealizedGainPercent,
  };
}
