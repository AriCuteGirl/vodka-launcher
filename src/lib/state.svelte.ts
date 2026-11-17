// App-wide reactive state. Components read `app.*` and call the functions below.
import { api } from "./api";
import { noteFriends } from "./alerts.svelte";
import { NO_OPTIONS, toAppError, type LaunchOptions, type Person, type Target, type Account, type AppError, type ExitEvent, type Game, type Instance, type Loadable, type MocktailStatus, type Settings, type User } from "./types";

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
  /** The active account's Robux; null while unknown. */
  robux: null as number | null,
  accounts: [] as Account[],
  instances: [] as Instance[],
  mocktail: null as MocktailStatus | null,
  online: typeof navigator === "undefined" ? true : navigator.onLine,
  search: "",
  toasts: [] as Toast[],
  crash: null as ExitEvent | null,
  /** A launch waiting for the user to accept the multi-instance warning. */
  /** The several-games warning: for a game about to start, or (name and run null) turned on from Settings. */
  pendingLaunch: null as { name: string | null; run: (() => Promise<unknown>) | null } | null,
  /** Accounts ticked on the Players page; the next Play or Join starts for all of them. */
  batch: [] as number[],
  /** The server and account picker, when open. `target` is set when the server is already decided (joining a friend). */
  launchDialog: null as { game: Game; target: Target | null; targetLabel: string | null } | null,
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

/** True where the game is run by the official Roblox client (Windows) instead of Mocktail. */
export const onWindows = () => app.mocktail?.engine === "roblox";

/** The active account's running game, if it has one. */
export function myInstance(): Instance | null {
  return app.instances.find((i) => i.accountId === app.account?.id) ?? null;
}

/** What a tile for `game` should show for the active account. */
export function phaseOf(game: Game): "idle" | "starting" | "playing" {
  const mine = myInstance();
  return mine && mine.game.universeId === game.universeId ? mine.phase : "idle";
}

/** Reads the active account's Robux balance. Quiet on failure: it's a convenience, not something to interrupt for. */
export async function refreshRobux() {
  const forAccount = app.account?.id;
  if (forAccount == null) {
    app.robux = null;
    return;
  }
  try {
    const amount = await api.robuxBalance();
    if (app.account?.id === forAccount) app.robux = amount;
  } catch {
    if (app.account?.id === forAccount) app.robux = null;
  }
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
  app.robux = null;
  void refreshRobux();
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
  app.batch = app.batch.filter((x) => x !== id);
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
    await refreshAccounts();
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
    noteFriends(
      "roblox",
      forAccount,
      friends.map((f) => ({
        id: f.id,
        name: f.displayName,
        avatarUrl: f.avatarUrl,
        activity: f.presence.kind === "in_game" ? "playing" : f.presence.kind === "offline" ? "offline" : "online",
        game: f.presence.kind === "in_game" ? f.presence.location : null,
        join: f.presence.gameId ? () => void joinFriend(f) : null,
        profile: `/users/${f.id}`,
      })),
    );
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

export async function joinFriend(friend: Person) {
  if (app.batch.length === 0) {
    return start(friend.presence.location ?? `${friend.displayName}'s game`, () => api.joinFriend(friend.id));
  }
  // Several accounts: find the server once, then let the picker send them all there.
  try {
    const at = await api.friendLocation(friend.id);
    app.launchDialog = { game: at.game, target: { kind: "server", placeId: at.placeId, gameId: at.gameId }, targetLabel: `${friend.displayName}'s server` };
  } catch (e) {
    fail(e);
  }
}

/** Plays as the active account, or opens the picker when several accounts are ticked. */
export function play(game: Game) {
  if (app.batch.length > 0) return openLaunchDialog(game);
  return start(game.name, () => api.launchGame(game));
}

export function openLaunchDialog(game: Game) {
  app.launchDialog = { game, target: null, targetLabel: null };
}

export function toggleBatch(accountId: number) {
  app.batch = app.batch.includes(accountId) ? app.batch.filter((id) => id !== accountId) : [...app.batch, accountId];
}

const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Starts `game` for each of `accountIds`, one after another, all in the same server when `target` is given. */
export async function launchMany(game: Game, accountIds: number[], target: Target | null) {
  const busy = new Set(app.instances.map((i) => i.accountId));
  const ids = accountIds.filter((id) => !busy.has(id));
  const skipped = accountIds.length - ids.length;
  if (skipped) toast(`${skipped} of those accounts ${skipped === 1 ? "is" : "are"} already in a game and ${skipped === 1 ? "was" : "were"} skipped.`);
  if (ids.length === 0) return;
  if (app.instances.length + ids.length > MAX_INSTANCES) {
    toast(`That would be more than ${MAX_INSTANCES} games at once. Untick some accounts or close a game.`, "danger");
    return;
  }
  const run = async () => {
    let started = 0;
    for (const id of ids) {
      const name = app.accounts.find((a) => a.id === id)?.displayName ?? "An account";
      try {
        await api.launchGame(game, id, target);
        started++;
      } catch (e) {
        toast(`${name}: ${toAppError(e).message}`, "danger");
      }
      await refreshInstances();
      // Give each game a moment to claim its slot and profile before the next one starts.
      if (id !== ids[ids.length - 1]) await pause(2500);
    }
    if (started > 1) toast(`Started ${game.name} for ${started} accounts.`, "ok");
  };
  const total = app.instances.length + ids.length;
  if (total > 1 && !app.settings.multiInstanceAccepted) {
    app.pendingLaunch = { name: `${game.name} for ${ids.length} account${ids.length === 1 ? "" : "s"}`, run };
    return;
  }
  if (total > 1) {
    toast(`Starting ${ids.length} game${ids.length === 1 ? "" : "s"} (${total} running in total). Together they need about ${(total * GAME_MEMORY_GB).toFixed(1)} GB of memory, and each one adds account risk.`, "danger", 5000);
  }
  await run();
}

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
  if (onWindows()) {
    toast(`Starting Roblox window ${count}. Running several is unofficial: each one adds account risk, and Roblox may block it in any update.`, "danger", 5000);
    return;
  }
  toast(`Starting game ${count}. Together they need about ${(count * GAME_MEMORY_GB).toFixed(1)} GB of memory, and each one adds account risk.`, "danger", 5000);
}

/** Shows the several-games warning from Settings, with no game waiting on it. */
export function askMultiInstance() {
  app.pendingLaunch = { name: null, run: null };
}

export async function acceptMultiInstance() {
  const pending = app.pendingLaunch;
  app.pendingLaunch = null;
  await setFlags({ multiInstanceAccepted: true });
  if (pending?.run && app.settings.multiInstanceAccepted) {
    warnAboutLoad();
    await launch(pending.run);
  } else if (app.settings.multiInstanceAccepted && onWindows()) {
    toast("Several games at once is on. If Roblox is open right now, close it first: Vodka has to start before Roblox for this to work.", "ok", 6000);
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
  if (event.rejoin === "signin") {
    // Not a crash: Roblox didn't answer the sign-in. Vodka is already trying again.
    toast(`Roblox's sign-in didn't answer for ${event.accountName}. Trying again in a few seconds.`);
  } else if (event.rejoin === "signin_limit") {
    app.crash = { ...event, report: { ...event.report, outcome: "crashed" } };
  } else if (event.rejoin === "crash") {
    toast(`${event.game.name} crashed for ${event.accountName}. Rejoining in a few seconds.`, "danger", 7000);
  } else if (event.rejoin === "disconnect") {
    toast(`${event.accountName} was disconnected from ${event.game.name}. Rejoining.`, "danger", 7000);
  } else if (event.rejoin === "limit") {
    // Five automatic rejoins in fifteen minutes means something is wrong; stop and say so.
    app.crash = {
      ...event,
      report: { ...event.report, outcome: "crashed", detail: `${event.report.detail ?? "The game keeps closing."} Vodka rejoined 5 times in 15 minutes and has stopped trying.` },
    };
  } else if (event.restarting) {
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
  // This is an app, not a web page: no browser menu with Back, Reload and the like.
  // Text boxes keep theirs, so copy and paste by mouse still work.
  window.addEventListener("contextmenu", (e) => {
    const target = e.target as HTMLElement | null;
    if (!target?.closest("input, textarea, [contenteditable]")) e.preventDefault();
  });
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
    app.robux = null;
    void refreshRobux();
    void loadHome();
    void refreshFriends();
  });
  await api.onLoginClosed(() => (app.signingIn = false));
  await api.onAuthError((error) => {
    app.signingIn = false;
    toast(error.message, "danger");
  });
  // Robux change when you buy, sell or get paid elsewhere; every two minutes is plenty.
  setInterval(() => app.account && app.online && void refreshRobux(), 120_000);
  // Friends come and go; once a minute keeps the rail fresh without leaning on Roblox.
  setInterval(() => app.account && app.online && void refreshFriends(true), 60_000);
  // You can switch games inside Roblox; ask where each playing account really is now and then.
  setInterval(async () => {
    if (app.instances.length === 0 || !app.online) return;
    try {
      if (await api.refreshInstanceGames()) await refreshInstances();
    } catch {
      // Not worth a message; the next round will try again.
    }
  }, 30_000);
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
  // Imported here, not at the top: the chat module itself imports this one.
  void import("./chat.svelte").then((m) => m.startChat());
}
