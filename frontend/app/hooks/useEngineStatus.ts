import { useQuery } from "@tanstack/react-query";
import { deskApi } from "../lib/api";
import { queryKeys } from "./query-keys";

export function useEngineStatus(strategyId: string | null, enabled = true) {
  return useQuery({
    queryKey: queryKeys.strategyExecutionState(strategyId),
    queryFn: async () => {
      if (!strategyId) {
        throw new Error("Strategy id is required.");
      }

      const [statesResponse, signalsResponse] = await Promise.all([
        deskApi.getStrategyRuntimeState(strategyId),
        deskApi.getStrategySignals(strategyId),
      ]);

      return {
        runtimeStates: statesResponse.states,
        strategySignals: signalsResponse.signals,
      };
    },
    enabled: enabled && Boolean(strategyId),
    staleTime: 5_000,
    refetchInterval: enabled && strategyId ? 7_500 : false,
  });
}
