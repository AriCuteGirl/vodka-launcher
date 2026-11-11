import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import adapter from "@sveltejs/adapter-static";

// Tauri has no Node server, so the UI is a static single-page app.
export default defineConfig({
  plugins: [
    sveltekit({
      adapter: adapter({ fallback: "index.html" }),
    }),
  ],
  resolve: {
    alias: { $lib: decodeURIComponent(new URL("./src/lib", import.meta.url).pathname) },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
