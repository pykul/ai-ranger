import { createContext, useContext, useState, type ReactNode } from "react";

type Days = 7 | 30 | 90;

interface TimeRangeContextValue {
  days: Days;
  setDays: (d: Days) => void;
}

const TimeRangeContext = createContext<TimeRangeContextValue | null>(null);

export function TimeRangeProvider({ children }: { children: ReactNode }) {
  const [days, setDays] = useState<Days>(7);
  return (
    <TimeRangeContext.Provider value={{ days, setDays }}>
      {children}
    </TimeRangeContext.Provider>
  );
}

export function useTimeRange(): TimeRangeContextValue {
  const ctx = useContext(TimeRangeContext);
  if (!ctx) throw new Error("useTimeRange must be used within TimeRangeProvider");
  return ctx;
}
