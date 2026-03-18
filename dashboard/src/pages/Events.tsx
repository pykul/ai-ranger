import { useState } from "react";
import { Search, ChevronDown, ChevronUp, ChevronLeft, ChevronRight } from "lucide-react";
import { useTimeRange } from "@/hooks/use-time-range";
import { useEvents } from "@/hooks/use-events";
import { formatProvider, formatProcess, formatDetection, formatOS, timeAgo } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { EventRow } from "@/lib/types";

type SortField = "timestamp" | "os_username" | "provider" | "process_name";

export default function Events() {
  const { days } = useTimeRange();
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [page, setPage] = useState(1);
  const [sort, setSort] = useState<SortField>("timestamp");
  const [order, setOrder] = useState<"asc" | "desc">("desc");
  const [expandedRow, setExpandedRow] = useState<number | null>(null);
  const limit = 25;

  // Simple debounce via timeout ref
  const [timer, setTimer] = useState<ReturnType<typeof setTimeout> | null>(null);
  function handleSearch(value: string) {
    setSearch(value);
    if (timer) clearTimeout(timer);
    setTimer(
      setTimeout(() => {
        setDebouncedSearch(value);
        setPage(1);
      }, 300)
    );
  }

  const events = useEvents({
    q: debouncedSearch,
    days,
    page,
    limit,
    sort,
    order,
  });

  const totalPages = events.data ? Math.ceil(events.data.total / limit) : 0;

  function toggleSort(field: SortField) {
    if (sort === field) {
      setOrder((o) => (o === "asc" ? "desc" : "asc"));
    } else {
      setSort(field);
      setOrder("desc");
    }
    setPage(1);
  }

  function SortIcon({ field }: { field: SortField }) {
    if (sort !== field) return null;
    return order === "asc" ? (
      <ChevronUp className="inline h-3 w-3" />
    ) : (
      <ChevronDown className="inline h-3 w-3" />
    );
  }

  return (
    <div>
      <h2 className="text-2xl font-semibold mb-6">Events</h2>

      {/* Search bar */}
      <div className="relative mb-6">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <input
          type="text"
          value={search}
          onChange={(e) => handleSearch(e.target.value)}
          placeholder="Search by provider, user, machine, process, IP address..."
          className="w-full rounded-lg border border-border bg-card pl-10 pr-4 py-3 text-sm outline-none focus:ring-2 focus:ring-ring"
        />
      </div>

      {/* Table */}
      <div className="rounded-lg border border-border bg-card overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border bg-muted/50">
              <Th field="timestamp" label="Time" sort={sort} toggleSort={toggleSort}>
                <SortIcon field="timestamp" />
              </Th>
              <Th field="os_username" label="User" sort={sort} toggleSort={toggleSort}>
                <SortIcon field="os_username" />
              </Th>
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">Machine</th>
              <Th field="provider" label="Provider" sort={sort} toggleSort={toggleSort}>
                <SortIcon field="provider" />
              </Th>
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">Host</th>
              <Th field="process_name" label="Tool" sort={sort} toggleSort={toggleSort}>
                <SortIcon field="process_name" />
              </Th>
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">OS</th>
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">Detection</th>
            </tr>
          </thead>
          <tbody>
            {events.data?.events?.map((event, i) => (
              <EventRowComponent
                key={`${event.timestamp}-${i}`}
                event={event}
                index={i}
                expanded={expandedRow === i}
                onToggle={() => setExpandedRow(expandedRow === i ? null : i)}
              />
            ))}
            {events.data?.events?.length === 0 && (
              <tr>
                <td colSpan={8} className="px-4 py-12 text-center text-muted-foreground">
                  No events found.
                </td>
              </tr>
            )}
            {!events.data?.events && !events.isLoading && (
              <tr>
                <td colSpan={8} className="px-4 py-12 text-center text-muted-foreground">
                  No events found.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between mt-4 text-sm text-muted-foreground">
          <span>
            Page {page} of {totalPages} ({events.data?.total.toLocaleString()} events)
          </span>
          <div className="flex gap-2">
            <button
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page <= 1}
              className="p-1.5 rounded border border-border hover:bg-muted disabled:opacity-30"
            >
              <ChevronLeft className="h-4 w-4" />
            </button>
            <button
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              disabled={page >= totalPages}
              className="p-1.5 rounded border border-border hover:bg-muted disabled:opacity-30"
            >
              <ChevronRight className="h-4 w-4" />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function Th({
  field,
  label,
  sort,
  toggleSort,
  children,
}: {
  field: SortField;
  label: string;
  sort: SortField;
  toggleSort: (f: SortField) => void;
  children: React.ReactNode;
}) {
  return (
    <th
      className={cn(
        "px-4 py-3 text-left font-medium cursor-pointer select-none transition-colors",
        sort === field ? "text-foreground" : "text-muted-foreground hover:text-foreground"
      )}
      onClick={() => toggleSort(field)}
    >
      {label} {children}
    </th>
  );
}

function EventRowComponent({
  event,
  index,
  expanded,
  onToggle,
}: {
  event: EventRow;
  index: number;
  expanded: boolean;
  onToggle: () => void;
}) {
  return (
    <>
      <tr
        className={cn(
          "border-b border-border cursor-pointer transition-colors hover:bg-muted/30",
          index % 2 === 0 ? "bg-card" : "bg-muted/10"
        )}
        onClick={onToggle}
      >
        <td className="px-4 py-3 text-muted-foreground">{timeAgo(event.timestamp)}</td>
        <td className="px-4 py-3">{event.os_username}</td>
        <td className="px-4 py-3 text-muted-foreground">{event.machine_hostname}</td>
        <td className="px-4 py-3">{formatProvider(event.provider)}</td>
        <td className="px-4 py-3 text-muted-foreground">{event.provider_host}</td>
        <td className="px-4 py-3">{formatProcess(event.process_name)}</td>
        <td className="px-4 py-3">
          <Badge variant="secondary">{formatOS(event.os_type)}</Badge>
        </td>
        <td className="px-4 py-3">
          <Badge variant="outline">{formatDetection(event.detection_method)}</Badge>
        </td>
      </tr>
      {expanded && (
        <tr className="border-b border-border bg-muted/20">
          <td colSpan={8} className="px-4 py-4">
            <div className="grid grid-cols-2 gap-x-8 gap-y-2 text-sm max-w-2xl">
              <Detail label="Provider host" value={event.provider_host} />
              <Detail label="Source IP" value={event.src_ip || "N/A"} />
              <Detail label="Process path" value={event.process_path || "N/A"} />
              <Detail label="Model hint" value={event.model_hint || "N/A"} />
              <Detail label="Capture mode" value={event.capture_mode.toUpperCase()} />
              <Detail label="Timestamp" value={new Date(event.timestamp).toLocaleString()} />
            </div>
          </td>
        </tr>
      )}
    </>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span className="text-muted-foreground">{label}: </span>
      <span>{value}</span>
    </div>
  );
}

function Badge({
  variant,
  children,
}: {
  variant: "secondary" | "outline";
  children: React.ReactNode;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded px-1.5 py-0.5 text-xs font-medium",
        variant === "secondary" && "bg-secondary text-secondary-foreground",
        variant === "outline" && "border border-border text-muted-foreground"
      )}
    >
      {children}
    </span>
  );
}
