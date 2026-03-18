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

interface TokenCreateResult {
  id: string;
  token: string;
  org_id: string;
  max_uses: number;
}

function TokensTab() {
  const queryClient = useQueryClient();
  const tokens = useQuery({
    queryKey: ["tokens"],
    queryFn: () => api<TokenRow[]>("/v1/admin/tokens"),
  });
  const fleet = useQuery({
    queryKey: ["fleet"],
    queryFn: () => api<FleetAgent[]>("/v1/dashboard/fleet"),
  });
  const deleteToken = useMutation({
    mutationFn: (id: string) =>
      api(`/v1/admin/tokens/${id}`, { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["tokens"] }),
  });

  const [showForm, setShowForm] = useState(false);
  const [label, setLabel] = useState("");
  const [maxUses, setMaxUses] = useState("10");
  const [formError, setFormError] = useState("");
  const [createdToken, setCreatedToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  // Resolve org_id from existing tokens or fleet agents.
  const orgId =
    tokens.data?.[0]?.OrgID ?? fleet.data?.[0]?.OrgID ?? null;

  const createToken = useMutation({
    mutationFn: (body: { org_id: string; label?: string; max_uses: number }) =>
      api<TokenCreateResult>("/v1/admin/tokens", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ["tokens"] });
      setCreatedToken(data.token);
      setLabel("");
      setMaxUses("10");
      setFormError("");
    },
    onError: (err: Error) => {
      setFormError(err.message || "Failed to create token");
    },
  });

  function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    setFormError("");

    if (!orgId) {
      setFormError("No organization found. Enroll at least one agent first.");
      return;
    }

    const uses = parseInt(maxUses, 10);
    if (isNaN(uses) || uses < 1) {
      setFormError("Max uses must be a number greater than 0.");
      return;
    }

    createToken.mutate({
      org_id: orgId,
      label: label.trim() || undefined,
      max_uses: uses,
    });
  }

  function handleCopy() {
    if (createdToken) {
      navigator.clipboard.writeText(createdToken);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }

  function handleDismissToken() {
    setCreatedToken(null);
    setShowForm(false);
    setCopied(false);
  }

  return (
    <div>
      {/* Created token banner -- shown once, cannot be retrieved later */}
      {createdToken && (
        <div className="mb-4 rounded-lg border border-green-300 bg-green-50 p-4">
          <p className="text-sm font-medium text-green-800 mb-2">
            Token created. Copy it now -- it will not be shown again.
          </p>
          <div className="flex items-center gap-2">
            <code className="flex-1 rounded bg-white px-3 py-2 text-sm font-mono border border-green-200 select-all break-all">
              {createdToken}
            </code>
            <button
              onClick={handleCopy}
              className="shrink-0 rounded bg-green-700 px-3 py-2 text-sm text-white hover:bg-green-800"
            >
              {copied ? "Copied" : "Copy"}
            </button>
          </div>
          <button
            onClick={handleDismissToken}
            className="mt-2 text-xs text-green-700 hover:underline"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Create token button / form */}
      {!createdToken && (
        <div className="mb-4">
          {!showForm ? (
            <button
              onClick={() => setShowForm(true)}
              className="rounded bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
            >
              Create Token
            </button>
          ) : (
            <form
              onSubmit={handleCreate}
              className="rounded-lg border border-border bg-card p-4 space-y-3"
            >
              <div>
                <label className="block text-sm font-medium text-muted-foreground mb-1">
                  Label (optional)
                </label>
                <input
                  type="text"
                  value={label}
                  onChange={(e) => setLabel(e.target.value)}
                  maxLength={100}
                  placeholder="e.g. Engineering team"
                  className="w-full rounded border border-border bg-background px-3 py-2 text-sm"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-muted-foreground mb-1">
                  Max uses
                </label>
                <input
                  type="number"
                  value={maxUses}
                  onChange={(e) => setMaxUses(e.target.value)}
                  min={1}
                  max={1000000}
                  className="w-32 rounded border border-border bg-background px-3 py-2 text-sm"
                />
              </div>
              {formError && (
                <p className="text-sm text-destructive">{formError}</p>
              )}
              <div className="flex gap-2">
                <button
                  type="submit"
                  disabled={createToken.isPending}
                  className="rounded bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
                >
                  {createToken.isPending ? "Creating..." : "Create"}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowForm(false);
                    setFormError("");
                  }}
                  className="rounded border border-border px-4 py-2 text-sm hover:bg-muted"
                >
                  Cancel
                </button>
              </div>
            </form>
          )}
        </div>
      )}

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
