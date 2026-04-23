import type { PaperAccountSummaryResponse } from "./api";

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
