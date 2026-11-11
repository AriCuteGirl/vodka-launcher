// The only place the UI talks to the Rust backend.
import type { Account, AppError, ExitEvent, Game, Instance, LaunchOptions, MocktailStatus, Settings, User } from "./types";

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
  beginLogin: () => call<void>("begin_login"),
  currentAccount: () => call<User | null>("current_account"),
  listAccounts: () => call<Account[]>("list_accounts"),
  switchAccount: (accountId: number) => call<void>("switch_account", { accountId }),
  removeAccount: (accountId: number) => call<void>("remove_account", { accountId }),
  continuePlaying: () => call<Game[]>("continue_playing"),
  favorites: () => call<Game[]>("favorites"),
  launchGame: (game: Game) => call<number>("launch_game", { game }),
  listInstances: () => call<Instance[]>("list_instances"),
  stopInstance: (id: number, restart: boolean) => call<void>("stop_instance", { id, restart }),
  getSettings: () => call<Settings>("get_settings"),
  setFlags: (flags: { onboarded?: boolean; multiInstanceAccepted?: boolean }) => call<Settings>("set_flags", { flags }),
  checkFflag: (name: string, value: string) => call<[string, string]>("check_fflag", { name, value }),
  saveLaunchOptions: (accountId: number | null, options: LaunchOptions | null) => call<Settings>("save_launch_options", { accountId, options }),
  rollbackClient: (accountId: number) => call<Settings>("rollback_client", { accountId }),
  onInstancesChanged: (h: () => void) => on<null>("instances://changed", h),
  onInstanceExit: (h: (event: ExitEvent) => void) => on<ExitEvent>("instances://exit", h),
  onInstanceError: (h: (error: AppError) => void) => on<AppError>("instances://error", h),
  onSignedIn: (h: (user: User) => void) => on<User>("auth://signed-in", h),
  onLoginClosed: (h: () => void) => on<null>("auth://closed", h),
  onAuthError: (h: (error: AppError) => void) => on<AppError>("auth://error", h),
};
