import { useQuery } from "@tanstack/react-query";
import { deskApi } from "../lib/api";
import { queryKeys } from "./query-keys";

export function usePaperAccounts() {
  return useQuery({
    queryKey: queryKeys.paperAccounts,
    queryFn: () => deskApi.listPaperAccounts(),
    staleTime: 30_000,
    gcTime: 5 * 60_000,
  });
}
