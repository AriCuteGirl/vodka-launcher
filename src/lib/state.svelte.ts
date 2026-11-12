// App-wide reactive state. Components read `app.*` and call the functions below.
import { api } from "./api";
import { NO_OPTIONS, toAppError, type LaunchOptions, type Person, type Account, type AppError, type ExitEvent, type Game, type Instance, type Loadable, type MocktailStatus, type Settings, type User } from "./types";

export interface Toast {
  id: number;
  tone: "info" | "ok" | "danger";
  text: string;
}

/** Vodka runs at most this many games at once (one per account). */
export const MAX_INSTANCES = 10;
/** Roughly what one running game needs. */
const GAME_MEMORY_GB = 1.5;

export const app = $state({
  booted: false,
  bootError: null as AppError | null,
  settings: { onboarded: false, activeAccount: null, multiInstanceAccepted: false, defaults: { ...NO_OPTIONS }, accountOptions: {} } as Settings,
  /** The active account, confirmed by Roblox. */
  account: null as User | null,
  accounts: [] as Account[],
  instances: [] as Instance[],
  mocktail: null as MocktailStatus | null,
  online: typeof navigator === "undefined" ? true : navigator.onLine,
  search: "",
  toasts: [] as Toast[],
  crash: null as ExitEvent | null,
  /** A launch waiting for the user to accept the multi-instance warning. */
  pendingLaunch: null as { name: string; run: () => Promise<unknown> } | null,
  friends: { status: "loading" } as Loadable<Person[]>,
  friendRequests: [] as Person[],
  signingIn: false,
  continuePlaying: { status: "loading" } as Loadable<Game[]>,
  favorites: { status: "loading" } as Loadable<Game[]>,
});

let nextToast = 1;

export function toast(text: string, tone: Toast["tone"] = "info", ms = tone === "danger" ? 8000 : 4500) {
  const id = nextToast++;
  app.toasts.push({ id, tone, text });
  setTimeout(() => dismissToast(id), ms);
}

export function dismissToast(id: number) {
  app.toasts = app.toasts.filter((t) => t.id !== id);
}

const fail = (e: unknown) => toast(toAppError(e).message, "danger");

/** The active account's running game, if it has one. */
export function myInstance(): Instance | null {
  return app.instances.find((i) => i.accountId === app.account?.id) ?? null;
}

/** What a tile for `game` should show for the active account. */
export function phaseOf(game: Game): "idle" | "starting" | "playing" {
  const mine = myInstance();
  return mine && mine.game.universeId === game.universeId ? mine.phase : "idle";
}

export async function refreshMocktail() {
  app.mocktail = await api.mocktailStatus();
}

export async function refreshAccounts() {
  try {
    app.accounts = await api.listAccounts();
  } catch (e) {
    fail(e);
  }
}

export async function refreshInstances() {
  try {
    app.instances = await api.listInstances();
  } catch (e) {
    fail(e);
  }
}

export async function loadHome() {
  if (!app.account) return;
  app.continuePlaying = { status: "loading" };
  app.favorites = { status: "loading" };
  const forAccount = app.account.id;
  const settle = async (load: () => Promise<Game[]>): Promise<Loadable<Game[]> | null> => {
    try {
      const data = await load();
      return app.account?.id === forAccount ? { status: "ready", data } : null;
    } catch (e) {
      if (app.account?.id !== forAccount) return null;
      return { status: "error", error: toAppError(e) };
    }
  };
  await Promise.all([
    settle(api.continuePlaying).then((r) => {
      if (r) app.continuePlaying = r;
    }),
    settle(api.favorites).then((r) => {
      if (r) app.favorites = r;
    }),
  ]);
}

export async function setFlags(flags: { onboarded?: boolean; multiInstanceAccepted?: boolean }) {
  try {
    app.settings = await api.setFlags(flags);
  } catch (e) {
    fail(e);
  }
}

/** Opens Roblox's login window. Also how another account is added. */
export async function signIn() {
  app.signingIn = true;
  try {
    await api.beginLogin();
  } catch (e) {
    app.signingIn = false;
    fail(e);
  }
}

async function reloadActiveAccount() {
  try {
    app.account = await api.currentAccount();
  } catch (e) {
    const error = toAppError(e);
    // An expired session still shows the shell; Home explains and offers sign-in.
    if (error.kind !== "session_expired") fail(e);
    app.account = null;
  }
  await refreshAccounts();
  void loadHome();
  void refreshFriends();
}

export async function switchAccount(id: number) {
  if (app.account?.id === id) return;
  try {
    await api.switchAccount(id);
    await reloadActiveAccount();
    if (app.account) toast(`Now using ${app.account.displayName}.`, "ok");
    else toast("That account's session has expired. Sign in to it again.", "danger");
  } catch (e) {
    fail(e);
  }
}

export async function removeAccount(id: number) {
  const name = app.accounts.find((a) => a.id === id)?.displayName ?? "Account";
  try {
    await api.removeAccount(id);
    await reloadActiveAccount();
    toast(`${name} was signed out and removed from this computer.`, "ok");
  } catch (e) {
    fail(e);
  }
}

async function launch(run: () => Promise<unknown>) {
  try {
    await run();
    await refreshInstances();
  } catch (e) {
    const error = toAppError(e);
    if (error.kind === "mocktail_missing" || error.kind === "flatpak_missing") await refreshMocktail();
    toast(error.message, "danger");
  }
}

/** Friends and pending requests of the active account. `quiet` keeps the current list on screen while refreshing. */
export async function refreshFriends(quiet = false) {
  if (!app.account) {
    app.friends = { status: "ready", data: [] };
    app.friendRequests = [];
    return;
  }
  const forAccount = app.account.id;
  if (!quiet) app.friends = { status: "loading" };
  try {
    const [friends, requests] = await Promise.all([api.listFriends(), api.listFriendRequests().catch(() => [] as Person[])]);
    if (app.account?.id !== forAccount) return;
    app.friends = { status: "ready", data: friends };
    app.friendRequests = requests;
  } catch (e) {
    // A failed background refresh shouldn't replace a list that is already showing.
    if (app.account?.id === forAccount && !(quiet && app.friends.status === "ready")) app.friends = { status: "error", error: toAppError(e) };
  }
}

export async function answerFriendRequest(person: Person, accept: boolean) {
  try {
    await api.answerFriendRequest(person.id, accept);
    toast(accept ? `You and ${person.displayName} are now friends.` : `Declined ${person.displayName}'s request.`, "ok");
  } catch (e) {
    fail(e);
  }
  await refreshFriends(true);
}

export const joinFriend = (friend: Person) =>
  start(friend.presence.location ?? `${friend.displayName}'s game`, () => api.joinFriend(friend.id));

export const play = (game: Game) => start(game.name, () => api.launchGame(game));

/** Runs the checks every launch goes through, then starts it. */
async function start(name: string, run: () => Promise<unknown>) {
  if (!app.account) return;
  const mine = myInstance();
  if (mine) {
    toast(
      mine.phase === "starting"
        ? `${mine.game.name} is still starting. Give it a moment.`
        : `${app.account.displayName} is in ${mine.game.name}. Close it, or switch account to start a second game.`,
    );
    return;
  }
  if (app.instances.length >= MAX_INSTANCES) {
    toast(`Vodka runs at most ${MAX_INSTANCES} games at once. Close one first.`);
    return;
  }
  if (app.instances.length > 0 && !app.settings.multiInstanceAccepted) {
    app.pendingLaunch = { name, run };
    return;
  }
  warnAboutLoad();
  await launch(run);
}

/** A five-second heads-up each time another game is added on top of running ones. */
function warnAboutLoad() {
  const count = app.instances.length + 1;
  if (count < 2) return;
  toast(`Starting game ${count}. Together they need about ${(count * GAME_MEMORY_GB).toFixed(1)} GB of memory, and each one adds account risk.`, "danger", 5000);
}

export async function acceptMultiInstance() {
  const pending = app.pendingLaunch;
  app.pendingLaunch = null;
  await setFlags({ multiInstanceAccepted: true });
  if (pending && app.settings.multiInstanceAccepted) {
    warnAboutLoad();
    await launch(pending.run);
  }
}

/** Saves launch options for all accounts (`accountId` null) or one. Returns the error text if they were rejected. */
export async function saveLaunchOptions(accountId: number | null, options: LaunchOptions | null): Promise<string | null> {
  try {
    app.settings = await api.saveLaunchOptions(accountId, options);
    return null;
  } catch (e) {
    return toAppError(e).message;
  }
}

export async function rollbackClient(accountId: number) {
  try {
    app.settings = await api.rollbackClient(accountId);
    await refreshAccounts();
    const version = app.accounts.find((a) => a.id === accountId)?.clientVersion;
    toast(`Rolled back to Roblox ${version ?? "the previous client"}. Updates are now held for this account.`, "ok");
  } catch (e) {
    fail(e);
  }
}

export async function stopInstance(id: number, restart = false) {
  try {
    await api.stopInstance(id, restart);
  } catch (e) {
    fail(e);
    await refreshInstances();
  }
}

function onExit(event: ExitEvent) {
  if (event.restarting) {
    toast(`Restarting ${event.game.name}…`);
  } else if (event.report.outcome === "forwarded") {
    toast(`Sent ${event.game.name} to a Mocktail window that was already open.`, "ok");
  } else if (event.report.outcome === "crashed") {
    app.crash = event;
  }
  void refreshMocktail();
  void refreshAccounts();
}

/** Loads everything the shell needs and wires backend events. Call once. */
export async function boot() {
  window.addEventListener("online", () => (app.online = true));
  window.addEventListener("offline", () => (app.online = false));
  await api.onInstancesChanged(() => {
    void refreshInstances();
    void refreshMocktail();
  });
  await api.onInstanceExit(onExit);
  await api.onInstanceError((error) => toast(error.message, "danger"));
  await api.onSignedIn((user) => {
    app.signingIn = false;
    app.account = user;
    toast(`Signed in as ${user.displayName}.`, "ok");
    void refreshAccounts();
    void loadHome();
    void refreshFriends();
  });
  await api.onLoginClosed(() => (app.signingIn = false));
  await api.onAuthError((error) => {
    app.signingIn = false;
    toast(error.message, "danger");
  });
  // Friends come and go; once a minute keeps the rail fresh without leaning on Roblox.
  setInterval(() => app.account && app.online && void refreshFriends(true), 60_000);
  // CPU and memory only change while something runs.
  setInterval(() => app.instances.length > 0 && void refreshInstances(), 2000);
  try {
    const [settings, mocktail] = await Promise.all([api.getSettings(), api.mocktailStatus()]);
    app.settings = settings;
    app.mocktail = mocktail;
    await Promise.all([reloadActiveAccount(), refreshInstances()]);
  } catch (e) {
    app.bootError = toAppError(e);
  }
  app.booted = true;
}
