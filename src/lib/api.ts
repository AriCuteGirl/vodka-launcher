// The only place the UI talks to the Rust backend.
import type { ExitReport, Game, MocktailStatus, Settings, User } from "./types";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  if (import.meta.env.DEV) {
    const { demoInvoke } = await import("./demo");
    return demoInvoke(cmd, args) as Promise<T>;
  }
  throw { kind: "other", message: "Vodka Launcher has to run as a desktop app." };
}

async function on<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, (e) => handler(e.payload));
  }
  if (import.meta.env.DEV) {
    const { demoListen } = await import("./demo");
    return demoListen(event, (e) => handler(e.payload as T));
  }
  return () => {};
}

export const api = {
  mocktailStatus: () => call<MocktailStatus>("mocktail_status"),
  launchGame: (placeId: number) => call<void>("launch_game", { placeId }),
  sessionLog: () => call<string[]>("session_log"),
  beginLogin: () => call<void>("begin_login"),
  currentAccount: () => call<User | null>("current_account"),
  signOut: () => call<void>("sign_out"),
  continuePlaying: () => call<Game[]>("continue_playing"),
  favorites: (userId: number) => call<Game[]>("favorites", { userId }),
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<void>("save_settings", { settings }),
  onLog: (h: (line: string) => void) => on<string>("mocktail://log", h),
  onExit: (h: (report: ExitReport) => void) => on<ExitReport>("mocktail://exit", h),
  onSignedIn: (h: (user: User) => void) => on<User>("auth://signed-in", h),
  onLoginClosed: (h: () => void) => on<null>("auth://closed", h),
  onAuthError: (h: (error: { kind: string; message: string }) => void) => on("auth://error", h),
};
