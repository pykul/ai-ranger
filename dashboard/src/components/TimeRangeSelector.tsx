import { useTimeRange } from "@/hooks/use-time-range";
import { cn } from "@/lib/utils";

const options = [7, 30, 90] as const;
const labels: Record<number, string> = { 7: "7d", 30: "30d", 90: "90d" };

export default function TimeRangeSelector() {
  const { days, setDays } = useTimeRange();

  return (
    <div className="flex gap-1 rounded-md border border-border p-0.5">
      {options.map((d) => (
        <button
          key={d}
          onClick={() => setDays(d)}
          className={cn(
            "px-3 py-1 text-xs font-medium rounded transition-colors",
            days === d
              ? "bg-primary text-primary-foreground"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          {labels[d]}
        </button>
      ))}
    </div>
  );
}
