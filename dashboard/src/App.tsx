import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AuthProvider, useAuth } from "@/hooks/use-auth";
import DashboardLayout from "@/layouts/DashboardLayout";
import Login from "@/pages/Login";
import Overview from "@/pages/Overview";
import Fleet from "@/pages/Fleet";
import Providers from "@/pages/Providers";
import Users from "@/pages/Users";
import Events from "@/pages/Events";
import Tokens from "@/pages/Tokens";

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

  // state === "dev" or "authenticated"
  return (
    <Routes>
      <Route element={<DashboardLayout />}>
        <Route index element={<Overview />} />
        <Route path="fleet" element={<Fleet />} />
        <Route path="providers" element={<Providers />} />
        <Route path="users" element={<Users />} />
        <Route path="events" element={<Events />} />
        <Route path="tokens" element={<Tokens />} />
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
          <AppRoutes />
        </AuthProvider>
      </BrowserRouter>
    </QueryClientProvider>
  );
}
