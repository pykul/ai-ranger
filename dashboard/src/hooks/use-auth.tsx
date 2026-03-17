import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { isAuthRequired } from "@/lib/api";
import { getAccessToken, clearTokens } from "@/lib/auth";

type AuthState = "loading" | "authenticated" | "unauthenticated" | "dev";

interface AuthContextValue {
  state: AuthState;
  logout: () => void;
  setAuthenticated: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AuthState>("loading");

  useEffect(() => {
    isAuthRequired().then((required) => {
      if (!required) {
        setState("dev");
      } else if (getAccessToken()) {
        setState("authenticated");
      } else {
        setState("unauthenticated");
      }
    });
  }, []);

  const logout = () => {
    clearTokens();
    setState("unauthenticated");
  };

  const setAuthenticated = () => {
    setState("authenticated");
  };

  return (
    <AuthContext.Provider value={{ state, logout, setAuthenticated }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within AuthProvider");
  return ctx;
}
