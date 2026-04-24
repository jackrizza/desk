import { createContext, useCallback, useEffect, useMemo, useState } from "react";
import type { AppUiState, SelectedPortfolioAccount } from "./app-state";
import {
  APP_UI_STATE_STORAGE_KEY,
  DEFAULT_APP_UI_STATE,
} from "./app-state";
import { readStoredJson, subscribeStoredJson, writeStoredJson } from "./storage";

type AppStateContextValue = {
  uiState: AppUiState;
  setUiState: (nextState: Partial<AppUiState>) => void;
  setSelectedPortfolioAccount: (
    account: SelectedPortfolioAccount,
  ) => void;
  setSelectedPaperAccountId: (accountId: string | null) => void;
  setSelectedStrategyId: (strategyId: string | null) => void;
  setActiveStrategyTab: (tab: string) => void;
};

export const AppStateContext = createContext<AppStateContextValue | null>(null);

function selectedPortfolioAccountsEqual(
  left: SelectedPortfolioAccount,
  right: SelectedPortfolioAccount,
) {
  return left?.kind === right?.kind && left?.id === right?.id;
}

function appUiStatesEqual(left: AppUiState, right: AppUiState) {
  return (
    selectedPortfolioAccountsEqual(
      left.selectedPortfolioAccount,
      right.selectedPortfolioAccount,
    )
    && left.selectedPaperAccountId === right.selectedPaperAccountId
    && left.selectedStrategyId === right.selectedStrategyId
    && left.activeStrategyTab === right.activeStrategyTab
  );
}

export function AppStateProvider(props: { children: React.ReactNode }) {
  const [uiState, setUiStateValue] = useState<AppUiState>(() =>
    readStoredJson(APP_UI_STATE_STORAGE_KEY, DEFAULT_APP_UI_STATE),
  );

  useEffect(() => {
    return subscribeStoredJson(
      APP_UI_STATE_STORAGE_KEY,
      (nextState) => {
        setUiStateValue((current) =>
          appUiStatesEqual(current, nextState) ? current : nextState,
        );
      },
      DEFAULT_APP_UI_STATE,
    );
  }, []);

  useEffect(() => {
    writeStoredJson(APP_UI_STATE_STORAGE_KEY, uiState);
  }, [uiState]);

  const setUiState = useCallback((nextState: Partial<AppUiState>) => {
    setUiStateValue((current) => {
      const next = {
        ...current,
        ...nextState,
      };

      if (appUiStatesEqual(current, next)) {
        return current;
      }

      return next;
    });
  }, []);

  const setSelectedPortfolioAccount = useCallback(
    (account: SelectedPortfolioAccount) => {
      setUiStateValue((current) =>
        selectedPortfolioAccountsEqual(current.selectedPortfolioAccount, account)
          ? current
          : {
              ...current,
              selectedPortfolioAccount: account,
            },
      );
    },
    [],
  );

  const setSelectedPaperAccountId = useCallback((accountId: string | null) => {
    setUiStateValue((current) =>
      current.selectedPaperAccountId === accountId
        ? current
        : {
            ...current,
            selectedPaperAccountId: accountId,
          },
    );
  }, []);

  const setSelectedStrategyId = useCallback((strategyId: string | null) => {
    setUiStateValue((current) =>
      current.selectedStrategyId === strategyId
        ? current
        : {
            ...current,
            selectedStrategyId: strategyId,
          },
    );
  }, []);

  const setActiveStrategyTab = useCallback((tab: string) => {
    setUiStateValue((current) =>
      current.activeStrategyTab === tab
        ? current
        : {
            ...current,
            activeStrategyTab: tab,
          },
    );
  }, []);

  const value = useMemo<AppStateContextValue>(
    () => ({
      uiState,
      setUiState,
      setSelectedPortfolioAccount,
      setSelectedPaperAccountId,
      setSelectedStrategyId,
      setActiveStrategyTab,
    }),
    [
      setActiveStrategyTab,
      setSelectedPaperAccountId,
      setSelectedPortfolioAccount,
      setSelectedStrategyId,
      setUiState,
      uiState,
    ],
  );

  return (
    <AppStateContext.Provider value={value}>
      {props.children}
    </AppStateContext.Provider>
  );
}
