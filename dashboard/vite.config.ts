/// <reference types="vitest" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
  },
  server: {
    port: 3000,
    proxy: {
      "/api": {
        target: "http://localhost:8081",
        rewrite: (p) => p.replace(/^\/api/, ""),
      },
      "/ingest": {
        target: "http://localhost:8080",
        rewrite: (p) => p.replace(/^\/ingest/, ""),
      },
    },
  },
});
