import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { timeAgo, formatOS } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { FleetAgent } from "@/lib/types";

type Tab = "fleet" | "tokens";

export default function Admin() {
  const [tab, setTab] = useState<Tab>("fleet");

  return (
    <div>
      <h2 className="text-2xl font-semibold mb-6">Admin</h2>

      <div className="flex gap-1 mb-6 border-b border-border">
        <TabButton active={tab === "fleet"} onClick={() => setTab("fleet")}>
          Fleet
        </TabButton>
        <TabButton active={tab === "tokens"} onClick={() => setTab("tokens")}>
          Tokens
        </TabButton>
      </div>

      {tab === "fleet" && <FleetTab />}
      {tab === "tokens" && <TokensTab />}
    </div>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "px-4 py-2 text-sm font-medium border-b-2 transition-colors -mb-px",
        active
          ? "border-primary text-foreground"
          : "border-transparent text-muted-foreground hover:text-foreground"
      )}
    >
      {children}
    </button>
  );
}

// -- Fleet tab ----------------------------------------------------------------

function FleetTab() {
  const fleet = useQuery({
    queryKey: ["fleet"],
    queryFn: () => api<FleetAgent[]>("/v1/dashboard/fleet"),
  });
  const queryClient = useQueryClient();
  const revoke = useMutation({
    mutationFn: (id: string) =>
      api(`/v1/admin/agents/${id}`, { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["fleet"] }),
  });

  return (
    <div className="rounded-lg border border-border bg-card overflow-hidden">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border bg-muted/50">
            <th className="px-4 py-3 text-left font-medium text-muted-foreground">Hostname</th>
            <th className="px-4 py-3 text-left font-medium text-muted-foreground">User</th>
            <th className="px-4 py-3 text-left font-medium text-muted-foreground">OS</th>
            <th className="px-4 py-3 text-left font-medium text-muted-foreground">Status</th>
            <th className="px-4 py-3 text-left font-medium text-muted-foreground">Last seen</th>
            <th className="px-4 py-3 text-left font-medium text-muted-foreground">Enrolled</th>
            <th className="px-4 py-3 text-right font-medium text-muted-foreground">Actions</th>
          </tr>
        </thead>
        <tbody>
          {fleet.data?.map((agent) => (
            <tr key={agent.ID} className="border-b border-border hover:bg-muted/30">
              <td className="px-4 py-3">{agent.Hostname}</td>
              <td className="px-4 py-3 text-muted-foreground">{agent.OsUsername}</td>
              <td className="px-4 py-3">
                <span className="inline-flex items-center rounded px-1.5 py-0.5 text-xs font-medium bg-secondary text-secondary-foreground">
                  {formatOS(agent.Os)}
                </span>
              </td>
              <td className="px-4 py-3">
                <span
                  className={cn(
                    "inline-flex items-center rounded px-1.5 py-0.5 text-xs font-medium",
                    agent.Status === "active"
                      ? "bg-green-50 text-green-700"
                      : "bg-red-50 text-red-700"
                  )}
                >
                  {agent.Status === "active" ? "Active" : "Revoked"}
                </span>
              </td>
              <td className="px-4 py-3 text-muted-foreground">
                {agent.LastSeenAt ? timeAgo(agent.LastSeenAt) : "Never"}
              </td>
              <td className="px-4 py-3 text-muted-foreground">{timeAgo(agent.EnrolledAt)}</td>
              <td className="px-4 py-3 text-right">
                {agent.Status === "active" && (
                  <button
                    onClick={() => revoke.mutate(agent.ID)}
                    className="text-xs text-destructive hover:underline"
                  >
                    Revoke
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// -- Tokens tab ---------------------------------------------------------------

interface TokenRow {
  ID: string;
  OrgID: string;
  Label: string | null;
  MaxUses: number;
  UsedCount: number;
  CreatedAt: string;
}

function TokensTab() {
  const queryClient = useQueryClient();
  const tokens = useQuery({
    queryKey: ["tokens"],
    queryFn: () => api<TokenRow[]>("/v1/admin/tokens"),
  });
  const deleteToken = useMutation({
    mutationFn: (id: string) =>
      api(`/v1/admin/tokens/${id}`, { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["tokens"] }),
  });

  return (
    <div>
      <div className="rounded-lg border border-border bg-card overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border bg-muted/50">
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">Label</th>
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">Uses</th>
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">Created</th>
              <th className="px-4 py-3 text-right font-medium text-muted-foreground">Actions</th>
            </tr>
          </thead>
          <tbody>
            {tokens.data?.map((token) => (
              <tr key={token.ID} className="border-b border-border hover:bg-muted/30">
                <td className="px-4 py-3">{token.Label ?? "No label"}</td>
                <td className="px-4 py-3 text-muted-foreground">
                  {token.UsedCount} / {token.MaxUses === 0 ? "unlimited" : token.MaxUses}
                </td>
                <td className="px-4 py-3 text-muted-foreground">{timeAgo(token.CreatedAt)}</td>
                <td className="px-4 py-3 text-right">
                  <button
                    onClick={() => deleteToken.mutate(token.ID)}
                    className="text-xs text-destructive hover:underline"
                  >
                    Revoke
                  </button>
                </td>
              </tr>
            ))}
            {tokens.data?.length === 0 && (
              <tr>
                <td colSpan={4} className="px-4 py-12 text-center text-muted-foreground">
                  No enrollment tokens.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
