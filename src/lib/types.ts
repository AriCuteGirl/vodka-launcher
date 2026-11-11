export interface User {
  id: number;
  name: string;
  displayName: string;
  avatarUrl: string | null;
}

export interface Game {
  universeId: number;
  placeId: number;
  name: string;
  playing: number | null;
  iconUrl: string | null;
}

export interface InstallStep {
  label: string;
  command: string;
}

export interface MocktailStatus {
  flatpakAvailable: boolean;
  installed: boolean;
  version: string | null;
  robloxVersion: string | null;
  running: boolean;
  distro: string;
  installSteps: InstallStep[];
}

export interface Presence {
  kind: "offline" | "online" | "in_game" | "studio";
  location: string | null;
}

export interface Account {
  id: number;
  name: string;
  displayName: string;
  avatarUrl: string | null;
  active: boolean;
  presence: Presence | null;
  /** In a game that Vodka started. */
  inVodkaGame: boolean;
}

export interface Instance {
  id: number;
  slot: number;
  accountId: number;
  accountName: string;
  game: Game;
  phase: "starting" | "playing";
  startedAtMs: number;
  cpuPercent: number;
  memoryBytes: number;
  log: string[];
}

export interface ExitEvent {
  id: number;
  game: Game;
  accountName: string;
  report: ExitReport;
  log: string[];
  restarting: boolean;
}

export interface Settings {
  onboarded: boolean;
  activeAccount: number | null;
  multiInstanceAccepted: boolean;
}

export interface ExitReport {
  code: number | null;
  outcome: "closed" | "forwarded" | "crashed";
  detail: string | null;
}

export type ErrorKind =
  | "flatpak_missing"
  | "mocktail_missing"
  | "launch"
  | "not_signed_in"
  | "session_expired"
  | "rate_limited"
  | "offline"
  | "api"
  | "keyring"
  | "blocked_url"
  | "other";

export interface AppError {
  kind: ErrorKind;
  message: string;
}

/** Whatever was thrown, as an error the UI can show. */
export function toAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "kind" in e && "message" in e) return e as AppError;
  return { kind: "other", message: e instanceof Error ? e.message : String(e) };
}

export type Loadable<T> =
  | { status: "loading" }
  | { status: "ready"; data: T }
  | { status: "error"; error: AppError };
