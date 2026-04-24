import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  deskApi,
  type StrategyTradingConfig,
  type UpdateStrategyTradingConfigRequest,
} from "../lib/api";
import { queryKeys } from "./query-keys";

export function useStrategyTradingConfig(strategyId: string | null) {
  return useQuery({
    queryKey: queryKeys.strategyTradingConfig(strategyId),
    queryFn: () => {
      if (!strategyId) {
        throw new Error("Strategy id is required.");
      }

      return deskApi.getStrategyTradingConfig(strategyId);
    },
    enabled: Boolean(strategyId),
    staleTime: 5_000,
  });
}

export function useUpdateStrategyTradingConfig(strategyId: string | null) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: UpdateStrategyTradingConfigRequest) => {
      if (!strategyId) {
        throw new Error("Strategy id is required.");
      }

      return deskApi.updateStrategyTradingConfig(strategyId, input);
    },
    onSuccess: (savedConfig: StrategyTradingConfig) => {
      queryClient.setQueryData(
        queryKeys.strategyTradingConfig(strategyId),
        savedConfig,
      );
    },
  });
}
