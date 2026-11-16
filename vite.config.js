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
    // The app's webview must not reuse scripts from an earlier dev session: a restarted
    // dev server can't serve a component's styles until it has compiled that component again.
    headers: { "Cache-Control": "no-store" },
    // Vite's own module responses override that header, so the webview can still revalidate an old
    // component (304) without this server ever compiling it, and its styles then fail to load
    // ("Unknown word" overlay). Compiling every component at startup closes that gap.
    warmup: { clientFiles: ["./src/**/*.svelte"] },
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
