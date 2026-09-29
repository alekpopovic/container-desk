import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    // Browser assertions must not lose state to a development-server reload.
    hmr: mode !== "browser-test",
    watch:
      mode === "browser-test"
        ? null
        : { ignored: ["**/src-tauri/**", "**/codex/**"] },
  },
  build: { target: ["es2022", "safari17"] },
}));
