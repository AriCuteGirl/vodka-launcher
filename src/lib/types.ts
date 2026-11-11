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

export interface Settings {
  onboarded: boolean;
  shareSessionWithMocktail: boolean;
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
