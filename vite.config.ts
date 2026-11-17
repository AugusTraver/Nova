/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],

  // Opciones de Vite pensadas para `tauri dev` y `tauri build`:
  // 1. no limpiar la pantalla, así se ven los errores de Rust
  clearScreen: false,
  // 2. Tauri espera un puerto fijo: si está ocupado, falla
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. no vigilar `src-tauri` (eso lo recompila Cargo)
      ignored: ["**/src-tauri/**"],
    },
  },

  test: {
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
  },
}));
