import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// In development the Rust server runs on :8780 and Vite proxies API calls to it.
const backend = process.env.HOMEPAGE_BACKEND ?? "http://127.0.0.1:8780";

export default defineConfig({
  plugins: [svelte()],
  build: { target: "es2022", assetsInlineLimit: 0 },
  server: {
    port: 5173,
    proxy: {
      "/api": { target: backend, changeOrigin: false },
      "/icons": { target: backend, changeOrigin: false },
    },
  },
});
