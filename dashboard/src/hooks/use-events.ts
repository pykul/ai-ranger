import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { EventsResult } from "@/lib/types";

interface UseEventsParams {
  q: string;
  days: number;
  page: number;
  limit: number;
  sort: string;
  order: "asc" | "desc";
}

export function useEvents({ q, days, page, limit, sort, order }: UseEventsParams) {
  const params = new URLSearchParams({
    days: String(days),
    page: String(page),
    limit: String(limit),
    sort,
    order,
  });
  if (q) params.set("q", q);

  return useQuery({
    queryKey: ["events", q, days, page, limit, sort, order],
    queryFn: () => api<EventsResult>(`/v1/events?${params}`),
  });
}
