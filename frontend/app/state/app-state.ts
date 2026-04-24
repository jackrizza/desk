import type { TradingAccountKind } from "../lib/api";

export type SelectedPortfolioAccount = {
  kind: TradingAccountKind;
  id: string;
} | null;

export type AppUiState = {
  selectedPortfolioAccount: SelectedPortfolioAccount;
  selectedPaperAccountId: string | null;
  selectedStrategyId: string | null;
  activeStrategyTab: string;
};

export const APP_UI_STATE_STORAGE_KEY = "desk-app-ui-state";

export const DEFAULT_APP_UI_STATE: AppUiState = {
  selectedPortfolioAccount: { kind: "manual", id: "manual" },
  selectedPaperAccountId: null,
  selectedStrategyId: null,
  activeStrategyTab: "build",
};
