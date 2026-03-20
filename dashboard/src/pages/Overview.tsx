import { useState, useMemo } from "react";
import ProviderIcon from "@/components/ProviderIcon";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { useTimeRange } from "@/hooks/use-time-range";
import { useOverview, useProviders, useUsers, useMachines, useTraffic } from "@/hooks/use-dashboard";
import { formatProvider, formatNumber } from "@/lib/format";
import { PROVIDER_COLORS, TOTAL_LINE_COLOR } from "@/lib/theme";

export default function Overview() {
  const { days } = useTimeRange();
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null);

  const overview = useOverview(days);
  const providers = useProviders(days);
  const users = useUsers(days, selectedProvider);
  const machines = useMachines(days);
  const traffic = useTraffic(days);

  // Build chart data: always per-provider lines with different colors.
  // When a provider is selected, only that provider's line is shown.
  const chartData = useMemo(() => {
    if (!traffic.data || traffic.data.length === 0) return [];

    const byTime = new Map<string, Record<string, number>>();
    for (const pt of traffic.data) {
      if (selectedProvider && pt.provider !== selectedProvider) continue;
      const key = new Date(pt.timestamp).toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
      });
      if (!byTime.has(key)) byTime.set(key, {});
      const row = byTime.get(key)!;
      const name = formatProvider(pt.provider);
      row[name] = (row[name] ?? 0) + pt.connections;
    }
    return Array.from(byTime.entries()).map(([time, data]) => ({ time, ...data }));
  }, [traffic.data, selectedProvider]);

  // Stable color assignment: each provider always gets the same color based on
  // its rank in the full (unfiltered) provider list.
  const providerColorMap = useMemo(() => {
    const map = new Map<string, string>();
    if (providers.data) {
      providers.data.forEach((p, i) => {
        map.set(formatProvider(p.provider), PROVIDER_COLORS[i % PROVIDER_COLORS.length] ?? TOTAL_LINE_COLOR);
      });
    }
    return map;
  }, [providers.data]);

  const chartLines = useMemo(() => {
    if (!providers.data) return [];
    const list = selectedProvider
      ? providers.data.filter((p) => p.provider === selectedProvider)
      : providers.data;
    return list.map((p) => formatProvider(p.provider));
  }, [selectedProvider, providers.data]);

  const hasChartData = chartData.length > 0 && chartLines.some((line) =>
    chartData.some((row) => (row as Record<string, unknown>)[line] !== undefined)
  );

  function handleProviderClick(provider: string) {
    setSelectedProvider((prev) => (prev === provider ? null : provider));
  }

  function handleLegendClick(entry: { value: string }) {
    const raw = providers.data?.find(
      (p) => formatProvider(p.provider) === entry.value
    )?.provider;
    if (raw) handleProviderClick(raw);
  }

  const filteredProviders = selectedProvider
    ? providers.data?.filter((p) => p.provider === selectedProvider)
    : providers.data;

  return (
    <div>
      <h2 className="text-2xl font-semibold mb-6">Dashboard</h2>

      {/* Stat cards */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        <StatCard
          label="AI connections"
          value={overview.data ? formatNumber(overview.data.total_connections) : "-"}
        />
        <StatCard
          label="Active developers"
          value={overview.data ? formatNumber(overview.data.active_users) : "-"}
        />
        <StatCard
          label="Providers detected"
          value={overview.data ? String(overview.data.provider_count) : "-"}
        />
      </div>

      {/* Timeseries chart */}
      <div className="rounded-lg border border-border bg-card p-6 mb-8">
        <h3 className="text-sm font-medium text-muted-foreground mb-4">
          Connections over time
          {selectedProvider && (
            <span className="ml-2 text-foreground">
              - {formatProvider(selectedProvider)}
              <button
                onClick={() => setSelectedProvider(null)}
                className="ml-1 text-muted-foreground hover:text-foreground"
              >
                (show all)
              </button>
            </span>
          )}
        </h3>

        {hasChartData ? (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <XAxis
                dataKey="time"
                tick={{ fontSize: 12 }}
                tickLine={false}
                axisLine={false}
              />
              <YAxis
                tick={{ fontSize: 12 }}
                tickLine={false}
                axisLine={false}
                width={40}
              />
              <Tooltip />
              <Legend
                onClick={handleLegendClick}
                wrapperStyle={{ cursor: "pointer", fontSize: 12 }}
              />
              {chartLines.map((name) => (
                <Line
                  key={name}
                  type="monotone"
                  dataKey={name}
                  stroke={providerColorMap.get(name) ?? TOTAL_LINE_COLOR}
                  strokeWidth={2}
                  dot={false}
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        ) : (
          <div className="flex items-center justify-center h-[300px] text-muted-foreground text-sm">
            No data for this period.
          </div>
        )}
      </div>

      {/* Ranked lists */}
      <div className="grid grid-cols-3 gap-4">
        <div className="rounded-lg border border-border bg-card p-6">
          <h3 className="text-sm font-medium text-muted-foreground mb-4">Top providers</h3>
          <div className="space-y-3">
            {filteredProviders?.map((p) => (
              <div key={p.provider} className="flex justify-between items-center">
                <button
                  onClick={() => handleProviderClick(p.provider)}
                  className="text-sm hover:underline text-left inline-flex items-center gap-2"
                >
                  <ProviderIcon provider={p.provider} />
                  {formatProvider(p.provider)}
                </button>
                <span className="text-sm text-muted-foreground">
                  {formatNumber(p.connections)}
                </span>
              </div>
            ))}
            {(!filteredProviders || filteredProviders.length === 0) && (
              <p className="text-sm text-muted-foreground">No data for this period.</p>
            )}
          </div>
        </div>

        <div className="rounded-lg border border-border bg-card p-6">
          <h3 className="text-sm font-medium text-muted-foreground mb-4">
            Top users
            {selectedProvider && (
              <span className="font-normal"> - {formatProvider(selectedProvider)}</span>
            )}
          </h3>
          <div className="space-y-3">
            {users.data?.map((u) => (
              <div key={u.os_username} className="flex justify-between items-center">
                <span className="text-sm">{u.os_username}</span>
                <span className="text-sm text-muted-foreground">
                  {formatNumber(u.connections)}
                </span>
              </div>
            ))}
            {(!users.data || users.data.length === 0) && (
              <p className="text-sm text-muted-foreground">No data for this period.</p>
            )}
          </div>
        </div>

        <div className="rounded-lg border border-border bg-card p-6">
          <h3 className="text-sm font-medium text-muted-foreground mb-4">Top machines</h3>
          <div className="space-y-3">
            {machines.data?.map((m) => (
              <div key={m.machine_hostname} className="flex justify-between items-center">
                <span className="text-sm">{m.machine_hostname}</span>
                <span className="text-sm text-muted-foreground">
                  {formatNumber(m.connections)}
                </span>
              </div>
            ))}
            {(!machines.data || machines.data.length === 0) && (
              <p className="text-sm text-muted-foreground">No data for this period.</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-border bg-card p-6">
      <p className="text-sm text-muted-foreground">{label}</p>
      <p className="text-3xl font-semibold mt-1">{value}</p>
    </div>
  );
}
