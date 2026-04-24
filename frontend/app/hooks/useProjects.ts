import { useQuery } from "@tanstack/react-query";
import { deskApi } from "../lib/api";
import { queryKeys } from "./query-keys";

export function useProjects() {
  return useQuery({
    queryKey: queryKeys.projects,
    queryFn: () => deskApi.listProjects(),
    staleTime: 10_000,
    refetchInterval: 10_000,
  });
}
