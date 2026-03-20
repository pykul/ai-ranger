import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  OverviewStats,
  ProviderBreakdown,
  UserActivity,
  MachineActivity,
  TrafficPoint,
} from "@/lib/types";

export function useOverview(days: number) {
  return useQuery({
    queryKey: ["overview", days],
    queryFn: () => api<OverviewStats>(`/v1/dashboard/overview?days=${days}`),
  });
}

export function useProviders(days: number) {
  return useQuery({
    queryKey: ["providers", days],
    queryFn: () => api<ProviderBreakdown[]>(`/v1/dashboard/providers?days=${days}`),
  });
}

export function useUsers(days: number, provider: string | null) {
  const params = new URLSearchParams({ days: String(days) });
  if (provider) params.set("provider", provider);
  return useQuery({
    queryKey: ["users", days, provider],
    queryFn: () => api<UserActivity[]>(`/v1/dashboard/users?${params}`),
  });
}

export function useMachines(days: number) {
  return useQuery({
    queryKey: ["machines", days],
    queryFn: () => api<MachineActivity[]>(`/v1/dashboard/machines?days=${days}`),
  });
}

export function useTraffic(days: number) {
  return useQuery({
    queryKey: ["traffic", days],
    queryFn: () => api<TrafficPoint[]>(`/v1/dashboard/traffic/timeseries?days=${days}`),
  });
}
