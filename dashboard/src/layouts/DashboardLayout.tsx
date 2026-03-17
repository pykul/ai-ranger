import { NavLink, Outlet } from "react-router-dom";
import {
  LayoutDashboard,
  Monitor,
  Blocks,
  Users,
  Search,
  KeyRound,
  LogOut,
} from "lucide-react";
import { useAuth } from "@/hooks/use-auth";

const navItems = [
  { to: "/", icon: LayoutDashboard, label: "Overview" },
  { to: "/fleet", icon: Monitor, label: "Fleet" },
  { to: "/providers", icon: Blocks, label: "Providers" },
  { to: "/users", icon: Users, label: "Users" },
  { to: "/events", icon: Search, label: "Events" },
  { to: "/tokens", icon: KeyRound, label: "Tokens" },
];

export default function DashboardLayout() {
  const { state, logout } = useAuth();

  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <aside className="w-56 border-r border-sidebar-border bg-sidebar flex flex-col">
        <div className="p-4 border-b border-sidebar-border">
          <h1 className="text-lg font-semibold tracking-tight">AI Ranger</h1>
        </div>

        <nav className="flex-1 py-2">
          {navItems.map(({ to, icon: Icon, label }) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              className={({ isActive }) =>
                `flex items-center gap-3 px-4 py-2.5 text-sm transition-colors ${
                  isActive
                    ? "bg-sidebar-accent text-sidebar-accent-foreground font-medium"
                    : "text-sidebar-foreground/70 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
                }`
              }
            >
              <Icon className="h-4 w-4" />
              {label}
            </NavLink>
          ))}
        </nav>

        {state !== "dev" && (
          <div className="p-2 border-t border-sidebar-border">
            <button
              onClick={logout}
              className="flex items-center gap-3 px-4 py-2.5 text-sm text-sidebar-foreground/70 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground w-full rounded transition-colors"
            >
              <LogOut className="h-4 w-4" />
              Sign out
            </button>
          </div>
        )}
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto">
        <div className="p-8 max-w-7xl mx-auto">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
