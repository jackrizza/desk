import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  deskApi,
  type StrategyRiskConfig,
  type UpdateStrategyRiskConfigRequest,
} from "../lib/api";
import { queryKeys } from "./query-keys";

export function useStrategyRiskConfig(strategyId: string | null) {
  return useQuery({
    queryKey: queryKeys.strategyRiskConfig(strategyId),
    queryFn: () => {
      if (!strategyId) {
        throw new Error("Strategy id is required.");
      }

      return deskApi.getStrategyRiskConfig(strategyId);
    },
    enabled: Boolean(strategyId),
    staleTime: 10_000,
  });
}

export function useUpdateStrategyRiskConfig(strategyId: string | null) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: UpdateStrategyRiskConfigRequest) => {
      if (!strategyId) {
        throw new Error("Strategy id is required.");
      }

      return deskApi.updateStrategyRiskConfig(strategyId, input);
    },
    onSuccess: (savedConfig: StrategyRiskConfig) => {
      queryClient.setQueryData(queryKeys.strategyRiskConfig(strategyId), savedConfig);
    },
  });
}
