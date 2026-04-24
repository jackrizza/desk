import { useMemo } from "react";
import type { TradingAccountOption } from "../lib/api";
import { usePaperAccounts } from "./usePaperAccounts";
import { useQuery } from "@tanstack/react-query";
import { deskApi } from "../lib/api";
import { queryKeys } from "./query-keys";

const MANUAL_OPTION: TradingAccountOption = {
  id: "manual",
  name: "Manual Portfolio",
  kind: "manual",
  label: "Manual Portfolio",
  is_active: true,
};

export function usePortfolioAccounts() {
  const paperAccountsQuery = usePaperAccounts();
  const liveAccountsQuery = useQuery({
    queryKey: queryKeys.liveAccounts,
    queryFn: () => deskApi.listLiveAccounts(),
    staleTime: 60_000,
  });

  const accountOptions = useMemo<TradingAccountOption[]>(() => {
    const paperOptions = (paperAccountsQuery.data ?? []).map((account) => ({
      id: account.id,
      name: account.name,
      kind: "paper" as const,
      label: `[Paper] ${account.name}`,
      is_active: account.is_active,
    }));
    const liveOptions = (liveAccountsQuery.data ?? []).map((account) => ({
      id: account.id,
      name: account.name,
      kind: "live" as const,
      label: `[Live] ${account.name}`,
      is_active: account.is_active,
    }));

    return [MANUAL_OPTION, ...paperOptions, ...liveOptions];
  }, [liveAccountsQuery.data, paperAccountsQuery.data]);

  return {
    accountOptions,
    paperAccountsQuery,
    liveAccountsQuery,
  };
}
