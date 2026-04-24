import { useQuery } from "@tanstack/react-query";
import { deskApi } from "../lib/api";
import { queryKeys } from "./query-keys";

export function usePaperAccountSummary(accountId: string | null) {
  return useQuery({
    queryKey: queryKeys.paperAccountSummary(accountId),
    queryFn: () => {
      if (!accountId) {
        throw new Error("Paper account id is required.");
      }

      return deskApi.getPaperAccountSummary(accountId);
    },
    enabled: Boolean(accountId),
    staleTime: 10_000,
    refetchInterval: accountId ? 15_000 : false,
  });
}
