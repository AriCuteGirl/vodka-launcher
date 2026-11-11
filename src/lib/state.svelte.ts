// App-wide reactive state. Components read `app.*` and call the functions below.
import { api } from "./api";
import { toAppError, type AppError, type ExitReport, type Game, type Loadable, type MocktailStatus, type Settings, type User } from "./types";

export interface Toast {
  id: number;
  tone: "info" | "ok" | "danger";
  text: string;
}

export interface Playing {
  game: Game;
  startedAt: number;
  /** "starting" until Mocktail is drawing frames, then "playing". */
  phase: "starting" | "playing";
}

/** Mocktail prints this once the game is on screen. */
export function isRunningMarker(line: string): boolean {
  return line.includes("[perf] present_sample");
}

function markPlaying() {
  if (app.playing?.phase === "starting") {
    app.playing.phase = "playing";
    app.playing.startedAt = Date.now();
    void refreshMocktail();
  }
}

export const app = $state({
  booted: false,
  bootError: null as AppError | null,
  settings: { onboarded: false, shareSessionWithMocktail: false } as Settings,
  account: null as User | null,
  mocktail: null as MocktailStatus | null,
  online: typeof navigator === "undefined" ? true : navigator.onLine,
  search: "",
  toasts: [] as Toast[],
  playing: null as Playing | null,
  log: [] as string[],
  crash: null as { game: Game; report: ExitReport; log: string[] } | null,
  signingIn: false,
  continuePlaying: { status: "loading" } as Loadable<Game[]>,
  favorites: { status: "loading" } as Loadable<Game[]>,
});

let nextToast = 1;

export function toast(text: string, tone: Toast["tone"] = "info") {
  const id = nextToast++;
  app.toasts.push({ id, tone, text });
  setTimeout(() => dismissToast(id), tone === "danger" ? 8000 : 4500);
}

export function dismissToast(id: number) {
  app.toasts = app.toasts.filter((t) => t.id !== id);
}

export async function refreshMocktail() {
  app.mocktail = await api.mocktailStatus();
}

export async function loadHome() {
  const user = app.account;
  if (!user) return;
  app.continuePlaying = { status: "loading" };
  app.favorites = { status: "loading" };
  const settle = async (load: () => Promise<Game[]>): Promise<Loadable<Game[]>> => {
    try {
      return { status: "ready", data: await load() };
    } catch (e) {
      const error = toAppError(e);
      if (error.kind === "session_expired") app.account = null;
      return { status: "error", error };
    }
  };
  await Promise.all([
    settle(api.continuePlaying).then((r) => {
      app.continuePlaying = r;
    }),
    settle(() => api.favorites(user.id)).then((r) => {
      app.favorites = r;
    }),
  ]);
}

export async function saveSettings(patch: Partial<Settings>) {
  const next = { ...app.settings, ...patch };
  try {
    await api.saveSettings(next);
    app.settings = next;
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
}

export async function signIn() {
  app.signingIn = true;
  try {
    await api.beginLogin();
  } catch (e) {
    app.signingIn = false;
    toast(toAppError(e).message, "danger");
  }
}

export async function signOut() {
  try {
    await api.signOut();
    app.account = null;
    toast("Signed out. Your session was removed from the keyring.", "ok");
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
}

export async function play(game: Game) {
  if (app.playing) {
    toast(
      app.playing.phase === "starting"
        ? `${app.playing.game.name} is still starting. Give it a moment.`
        : `${app.playing.game.name} is running. Close it before starting another game.`,
    );
    return;
  }
  app.log = [];
  const session: Playing = { game, startedAt: Date.now(), phase: "starting" };
  app.playing = session;
  // Fallback in case a Mocktail update renames the log line we watch for.
  setTimeout(() => app.playing?.game === session.game && markPlaying(), 30_000);
  try {
    await api.launchGame(game.placeId);
  } catch (e) {
    app.playing = null;
    const error = toAppError(e);
    if (error.kind === "mocktail_missing" || error.kind === "flatpak_missing") await refreshMocktail();
    toast(error.message, "danger");
  }
}

function onExit(report: ExitReport) {
  const playing = app.playing;
  app.playing = null;
  if (!playing) return;
  if (report.outcome === "forwarded") {
    toast(`Sent ${playing.game.name} to the Mocktail window that's already open.`, "ok");
  } else if (report.outcome === "crashed") {
    app.crash = { game: playing.game, report, log: app.log.slice(-12) };
  }
  void refreshMocktail();
}

/** Loads everything the shell needs and wires backend events. Call once. */
export async function boot() {
  window.addEventListener("online", () => (app.online = true));
  window.addEventListener("offline", () => (app.online = false));
  await api.onLog((line) => {
    if (isRunningMarker(line)) markPlaying();
    app.log.push(line);
    if (app.log.length > 400) app.log.splice(0, app.log.length - 400);
  });
  await api.onExit(onExit);
  await api.onSignedIn((user) => {
    app.signingIn = false;
    app.account = user;
    toast(`Signed in as ${user.displayName}.`, "ok");
    void loadHome();
  });
  await api.onLoginClosed(() => (app.signingIn = false));
  await api.onAuthError((error) => {
    app.signingIn = false;
    toast(error.message, "danger");
  });
  try {
    const [settings, mocktail] = await Promise.all([api.getSettings(), api.mocktailStatus()]);
    app.settings = settings;
    app.mocktail = mocktail;
    try {
      app.account = await api.currentAccount();
    } catch (e) {
      // Offline or expired: the shell still opens, Home explains what's wrong.
      const error = toAppError(e);
      if (error.kind !== "session_expired") toast(error.message, "danger");
    }
  } catch (e) {
    app.bootError = toAppError(e);
  }
  app.booted = true;
  void loadHome();
}
