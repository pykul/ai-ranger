import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AuthProvider, useAuth } from "@/hooks/use-auth";
import { TimeRangeProvider } from "@/hooks/use-time-range";
import DashboardLayout from "@/layouts/DashboardLayout";
import Login from "@/pages/Login";
import Overview from "@/pages/Overview";
import Events from "@/pages/Events";
import Admin from "@/pages/Admin";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: 1,
    },
  },
});

function AppRoutes() {
  const { state } = useAuth();

  if (state === "loading") {
    return (
      <div className="min-h-screen flex items-center justify-center text-muted-foreground">
        Loading...
      </div>
    );
  }

  if (state === "unauthenticated") {
    return <Login />;
  }

  return (
    <Routes>
      <Route element={<DashboardLayout />}>
        <Route index element={<Overview />} />
        <Route path="events" element={<Events />} />
        <Route path="admin" element={<Admin />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AuthProvider>
          <TimeRangeProvider>
            <AppRoutes />
          </TimeRangeProvider>
        </AuthProvider>
      </BrowserRouter>
    </QueryClientProvider>
  );
}
