export const queryKeys = {
  projects: ["projects"] as const,
  portfolios: ["portfolios"] as const,
  paperAccounts: ["paper-accounts"] as const,
  liveAccounts: ["live-accounts"] as const,
  paperAccountSummary: (accountId: string | null) =>
    ["paper-account-summary", accountId] as const,
  strategyTradingConfig: (strategyId: string | null) =>
    ["strategy-trading-config", strategyId] as const,
  strategyRiskConfig: (strategyId: string | null) =>
    ["strategy-risk-config", strategyId] as const,
  strategyExecutionState: (strategyId: string | null) =>
    ["strategy-execution-state", strategyId] as const,
};
