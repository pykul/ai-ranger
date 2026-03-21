import { useState, useCallback, useEffect } from "react";
import ProviderIcon from "@/components/ProviderIcon";
import { useSearchParams } from "react-router-dom";
import { Search, ChevronDown, ChevronUp, ChevronLeft, ChevronRight } from "lucide-react";
import { useTimeRange } from "@/hooks/use-time-range";
import { useEvents } from "@/hooks/use-events";
import { formatProvider, formatProcess, formatDetection, formatOS, timeAgo } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { EventRow } from "@/lib/types";

type SortField = "timestamp" | "os_username" | "provider" | "process_name";

const PAGE_SIZE_OPTIONS = [10, 25, 50, 100] as const;
const DEFAULT_LIMIT = 25;
const DEFAULT_SORT: SortField = "timestamp";
const DEFAULT_ORDER: "asc" | "desc" = "desc";
const MAX_VISIBLE_PAGES = 7;

function isSortField(value: string): value is SortField {
  return ["timestamp", "os_username", "provider", "process_name"].includes(value);
}

function isOrder(value: string): value is "asc" | "desc" {
  return value === "asc" || value === "desc";
}

function isValidLimit(value: number): boolean {
  return (PAGE_SIZE_OPTIONS as readonly number[]).includes(value);
}

/**
 * Generate an array of page numbers to display in the pagination bar.
 * Uses -1 as a sentinel value for ellipsis positions.
 */
export function generatePageNumbers(currentPage: number, totalPages: number): number[] {
  if (totalPages <= MAX_VISIBLE_PAGES) {
    return Array.from({ length: totalPages }, (_, i) => i + 1);
  }

  const pages = new Set<number>();
  pages.add(1);
  pages.add(totalPages);
  pages.add(currentPage);
  if (currentPage > 1) pages.add(currentPage - 1);
  if (currentPage < totalPages) pages.add(currentPage + 1);

  const sorted = Array.from(pages).sort((a, b) => a - b);

  // Insert ellipsis sentinels (-1) where there are gaps
  const result: number[] = [];
  let prev = 0;
  for (const p of sorted) {
    if (prev > 0 && p - prev > 1) {
      result.push(-1);
    }
    result.push(p);
    prev = p;
  }

  return result;
}

interface PaginationBarProps {
  page: number;
  totalPages: number;
  total: number;
  limit: number;
  onPageChange: (page: number) => void;
  onLimitChange: (limit: number) => void;
}

export function PaginationBar({ page, totalPages, total, limit, onPageChange, onLimitChange }: PaginationBarProps) {
  const pageNumbers = generatePageNumbers(page, totalPages);

  return (
    <div className="flex flex-wrap items-center justify-between gap-3 text-sm text-muted-foreground">
      {/* Left: page size selector and total count */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <label htmlFor="page-size" className="whitespace-nowrap">Rows per page:</label>
          <select
            id="page-size"
            value={limit}
            onChange={(e) => onLimitChange(Number(e.target.value))}
            className="rounded border border-border bg-card px-2 py-1 text-sm outline-none focus:ring-2 focus:ring-ring"
          >
            {PAGE_SIZE_OPTIONS.map((size) => (
              <option key={size} value={size}>
                {size}
              </option>
            ))}
          </select>
        </div>
        <span>{total.toLocaleString()} events</span>
      </div>

      {/* Right: page navigation buttons */}
      {totalPages > 1 && (
        <div className="flex items-center gap-1">
          <button
            onClick={() => onPageChange(page - 1)}
            disabled={page <= 1}
            className="p-1.5 rounded border border-border hover:bg-muted disabled:opacity-30 disabled:cursor-not-allowed"
            aria-label="Previous page"
          >
            <ChevronLeft className="h-4 w-4" />
          </button>

          {pageNumbers.map((pageNum, idx) =>
            pageNum === -1 ? (
              <span key={`ellipsis-${idx}`} className="px-1.5 py-1 text-muted-foreground select-none">
                ...
              </span>
            ) : (
              <button
                key={pageNum}
                onClick={() => onPageChange(pageNum)}
                className={cn(
                  "min-w-[2rem] px-2 py-1 rounded border text-sm",
                  pageNum === page
                    ? "border-ring bg-muted font-medium text-foreground"
                    : "border-border hover:bg-muted text-muted-foreground"
                )}
              >
                {pageNum}
              </button>
            )
          )}

          <button
            onClick={() => onPageChange(page + 1)}
            disabled={page >= totalPages}
            className="p-1.5 rounded border border-border hover:bg-muted disabled:opacity-30 disabled:cursor-not-allowed"
            aria-label="Next page"
          >
            <ChevronRight className="h-4 w-4" />
          </button>
        </div>
      )}
    </div>
  );
}

export default function Events() {
  const { days } = useTimeRange();
  const [searchParams, setSearchParams] = useSearchParams();

  // Initialize state from URL query params with fallback defaults
  const initialPage = Math.max(1, Number(searchParams.get("page")) || 1);
  const initialLimitParam = Number(searchParams.get("limit"));
  const initialLimit = isValidLimit(initialLimitParam) ? initialLimitParam : DEFAULT_LIMIT;
  const initialSort = isSortField(searchParams.get("sort") || "") ? (searchParams.get("sort") as SortField) : DEFAULT_SORT;
  const initialOrder = isOrder(searchParams.get("order") || "") ? (searchParams.get("order") as "asc" | "desc") : DEFAULT_ORDER;
  const initialQuery = searchParams.get("q") || "";

  const [search, setSearch] = useState(initialQuery);
  const [debouncedSearch, setDebouncedSearch] = useState(initialQuery);
  const [page, setPage] = useState(initialPage);
  const [limit, setLimit] = useState(initialLimit);
  const [sort, setSort] = useState<SortField>(initialSort);
  const [order, setOrder] = useState<"asc" | "desc">(initialOrder);
  const [expandedRow, setExpandedRow] = useState<number | null>(null);

  // Sync state changes back to URL query params
  useEffect(() => {
    const params = new URLSearchParams();
    if (debouncedSearch) params.set("q", debouncedSearch);
    if (page !== 1) params.set("page", String(page));
    if (limit !== DEFAULT_LIMIT) params.set("limit", String(limit));
    if (sort !== DEFAULT_SORT) params.set("sort", sort);
    if (order !== DEFAULT_ORDER) params.set("order", order);
    setSearchParams(params, { replace: true });
  }, [debouncedSearch, page, limit, sort, order, setSearchParams]);

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

  const total = events.data?.total ?? 0;
  const totalPages = total > 0 ? Math.ceil(total / limit) : 0;

  function toggleSort(field: SortField) {
    if (sort === field) {
      setOrder((o) => (o === "asc" ? "desc" : "asc"));
    } else {
      setSort(field);
      setOrder("desc");
    }
    setPage(1);
  }

  const handlePageChange = useCallback((newPage: number) => {
    setPage(Math.max(1, Math.min(newPage, totalPages || 1)));
    setExpandedRow(null);
  }, [totalPages]);

  const handleLimitChange = useCallback((newLimit: number) => {
    setLimit(newLimit);
    setPage(1);
    setExpandedRow(null);
  }, []);

  return (
    <div>
      <h2 className="text-2xl font-semibold mb-6">Events</h2>

      {/* Search bar */}
      <div className="relative mb-4">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <input
          type="text"
          value={search}
          onChange={(e) => handleSearch(e.target.value)}
          placeholder="Search by provider, user, machine, process, IP address..."
          className="w-full rounded-lg border border-border bg-card pl-10 pr-4 py-3 text-sm outline-none focus:ring-2 focus:ring-ring"
        />
      </div>

      {/* Top pagination bar */}
      <div className="mb-4">
        <PaginationBar
          page={page}
          totalPages={totalPages}
          total={total}
          limit={limit}
          onPageChange={handlePageChange}
          onLimitChange={handleLimitChange}
        />
      </div>

      {/* Table */}
      <div className="rounded-lg border border-border bg-card overflow-x-auto">
        <table className="w-full text-sm table-fixed">
          <colgroup>
            <col className="w-[11%]" />{/* Time */}
            <col className="w-[10%]" />{/* User */}
            <col className="w-[11%]" />{/* Machine */}
            <col className="w-[15%]" />{/* Provider */}
            <col className="w-[20%]" />{/* Host */}
            <col className="w-[13%]" />{/* Tool */}
            <col className="w-[10%]" />{/* OS */}
            <col className="w-[10%]" />{/* Detection */}
          </colgroup>
          <thead>
            <tr className="border-b border-border bg-muted/50">
              <Th field="timestamp" label="Time" sort={sort} toggleSort={toggleSort}>
                <SortIcon field="timestamp" sort={sort} order={order} />
              </Th>
              <Th field="os_username" label="User" sort={sort} toggleSort={toggleSort}>
                <SortIcon field="os_username" sort={sort} order={order} />
              </Th>
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">Machine</th>
              <Th field="provider" label="Provider" sort={sort} toggleSort={toggleSort}>
                <SortIcon field="provider" sort={sort} order={order} />
              </Th>
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">Host</th>
              <Th field="process_name" label="Tool" sort={sort} toggleSort={toggleSort}>
                <SortIcon field="process_name" sort={sort} order={order} />
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

      {/* Bottom pagination bar */}
      <div className="mt-4">
        <PaginationBar
          page={page}
          totalPages={totalPages}
          total={total}
          limit={limit}
          onPageChange={handlePageChange}
          onLimitChange={handleLimitChange}
        />
      </div>
    </div>
  );
}

function SortIcon({
  field,
  sort,
  order,
}: {
  field: SortField;
  sort: SortField;
  order: "asc" | "desc";
}) {
  if (sort !== field) return null;
  return order === "asc" ? (
    <ChevronUp className="inline h-3 w-3" />
  ) : (
    <ChevronDown className="inline h-3 w-3" />
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
        <td className="px-4 py-3 text-muted-foreground truncate">{timeAgo(event.timestamp)}</td>
        <td className="px-4 py-3 truncate">{event.os_username}</td>
        <td className="px-4 py-3 text-muted-foreground truncate">{event.machine_hostname}</td>
        <td className="px-4 py-3 truncate">
          <span className="inline-flex items-center gap-1.5">
            <ProviderIcon provider={event.provider} />
            {formatProvider(event.provider)}
          </span>
        </td>
        <td className="px-4 py-3 text-muted-foreground truncate">{event.provider_host}</td>
        <td className="px-4 py-3 truncate">{formatProcess(event.process_name)}</td>
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
    <div className="min-w-0">
      <span className="text-muted-foreground">{label}: </span>
      <span className="break-all">{value}</span>
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
