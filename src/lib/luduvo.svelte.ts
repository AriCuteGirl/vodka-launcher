// Luduvo's part of Vodka: its own account, game lists and Play, separate from Roblox's state.
import { api } from "./api";
import { noteFriends } from "./alerts.svelte";
import { toast } from "./state.svelte";
import { toAppError, type Loadable, type LuduvoFriend, type LuduvoPlace, type LuduvoRunning, type LuduvoShelf, type LuduvoStatus, type LuduvoUser } from "./types";

export const luduvo = $state({
  /** True once the Luduvo section has loaded for the first time. */
  started: false,
  status: null as LuduvoStatus | null,
  user: null as LuduvoUser | null,
  /** Sign-in is waiting for the browser. */
  signingIn: false,
  discover: { status: "loading" } as Loadable<LuduvoShelf[]>,
  recent: { status: "loading" } as Loadable<LuduvoPlace[]>,
  /** The game being started, so its tile can say so. */
  starting: null as number | null,
  /** The game Vodka started and is watching, while it runs. */
  playing: null as LuduvoRunning | null,
  friends: { status: "loading" } as Loadable<LuduvoFriend[]>,
  requests: [] as LuduvoFriend[],
  /** Luduvo's own currency; null while unknown. */
  coins: null as number | null,
});

async function load<T>(set: (v: Loadable<T>) => void, fetch: () => Promise<T>) {
  try {
    set({ status: "ready", data: await fetch() });
  } catch (e) {
    set({ status: "error", error: toAppError(e) });
  }
}

export function loadDiscover() {
  luduvo.discover = { status: "loading" };
  return load((v) => {
    luduvo.discover = v;
  }, api.luduvoDiscover);
}

/** Friends, requests and balance: everything that changes while Vodka is open. */
export async function loadSocial() {
  if (!luduvo.user) return;
  try {
    const friends = await api.luduvoFriends();
    luduvo.friends = { status: "ready", data: friends };
    if (luduvo.user) {
      noteFriends(
        "luduvo",
        luduvo.user.id,
        friends.map((f) => ({
          id: f.id,
          name: f.displayName,
          avatarUrl: f.avatarUrl,
          activity: f.state === "in_game" ? "playing" : f.state === "online" ? "online" : "offline",
          game: f.state === "in_game" ? f.placeTitle : null,
          join: f.state === "in_game" && f.joinable ? () => void joinLuduvoFriend(f) : null,
          profile: `/luduvo/users/${f.id}`,
        })),
      );
    }
  } catch (e) {
    // Keep showing the last list if a refresh fails.
    if (luduvo.friends.status !== "ready") luduvo.friends = { status: "error", error: toAppError(e) };
  }
  try {
    luduvo.requests = await api.luduvoFriendRequests();
  } catch {
    // The list just stays as it was.
  }
  try {
    luduvo.coins = await api.luduvoBalance();
  } catch {
    // The balance just stays as it was.
  }
}

export function loadRecent() {
  if (!luduvo.user) return;
  luduvo.recent = { status: "loading" };
  return load((v) => {
    luduvo.recent = v;
  }, api.luduvoRecent);
}

async function refreshPlaying() {
  const was = luduvo.playing;
  try {
    luduvo.playing = await api.luduvoRunning();
  } catch {
    return;
  }
  // A game just closed: Luduvo now lists it under recently played.
  if (was && !luduvo.playing) void loadRecent();
}

/** Runs the first time the Luduvo section opens; later visits reuse what was loaded. */
export async function startLuduvo() {
  if (luduvo.started) return;
  luduvo.started = true;
  void loadDiscover();
  void api.onLuduvoChanged(() => void refreshPlaying());
  void refreshPlaying();
  try {
    luduvo.status = await api.luduvoStatus();
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
  try {
    luduvo.user = await api.luduvoAccount();
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
  void loadRecent();
  void loadSocial();
  // Friends come and go: look again every minute, like the Roblox side.
  setInterval(() => {
    if (document.visibilityState === "visible") void loadSocial();
  }, 60_000);
}

export async function luduvoSignIn() {
  if (luduvo.signingIn) return;
  luduvo.signingIn = true;
  try {
    luduvo.user = await api.luduvoSignIn();
    toast(`Signed in to Luduvo as ${luduvo.user.displayName}.`, "ok");
    void loadRecent();
    void loadSocial();
  } catch (e) {
    const err = toAppError(e);
    if (err.message !== "Sign-in cancelled.") toast(err.message, "danger");
  } finally {
    luduvo.signingIn = false;
  }
}

export function cancelLuduvoSignIn() {
  void api.luduvoCancelSignIn();
}

export async function luduvoSignOut() {
  try {
    await api.luduvoSignOut();
    luduvo.user = null;
    luduvo.recent = { status: "loading" };
    luduvo.friends = { status: "loading" };
    luduvo.requests = [];
    luduvo.coins = null;
    toast("Signed out of Luduvo.", "ok");
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
}

export async function playLuduvo(place: LuduvoPlace) {
  if (luduvo.starting != null) return;
  if (!luduvo.user) {
    toast("Sign in to Luduvo first, then press Play again.", "info");
    return;
  }
  if (luduvo.playing) {
    toast(`You're already playing ${luduvo.playing.title}. Close it first, then press Play.`, "info");
    return;
  }
  luduvo.starting = place.id;
  try {
    luduvo.playing = await api.luduvoPlay(place.id, place.title);
    toast(`Starting ${place.title}. Luduvo checks for updates first, so its window can take a moment.`, "ok", 7000);
  } catch (e) {
    toast(toAppError(e).message, "danger");
  } finally {
    // The tile keeps saying "Starting" briefly, since the window takes a moment to appear.
    setTimeout(() => (luduvo.starting = null), 4000);
  }
}

export async function joinLuduvoFriend(friend: LuduvoFriend) {
  if (luduvo.playing) {
    toast(`You're already playing ${luduvo.playing.title}. Close it first, then join ${friend.displayName}.`, "info");
    return;
  }
  const title = friend.placeTitle ?? `${friend.displayName}'s game`;
  try {
    luduvo.playing = await api.luduvoJoinFriend(friend.id, friend.placeId, title);
    toast(`Joining ${friend.displayName} in ${title}.`, "ok");
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
}

export async function answerLuduvoRequest(friend: LuduvoFriend, accept: boolean) {
  try {
    await api.luduvoAnswerRequest(friend.id, accept);
    luduvo.requests = luduvo.requests.filter((r) => r.id !== friend.id);
    toast(accept ? `You and ${friend.displayName} are now friends.` : `Declined ${friend.displayName}'s request.`, "ok");
    if (accept) void loadSocial();
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
}

export async function claimDaily() {
  try {
    const answer = await api.luduvoClaimDaily();
    const amount = answer && typeof answer === "object" ? (answer.amount ?? answer.reward ?? answer.coins) : null;
    toast(typeof amount === "number" ? `Daily reward claimed: ${amount} C$.` : "Daily reward claimed.", "ok");
    luduvo.coins = await api.luduvoBalance().catch(() => luduvo.coins);
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
}

/** Joins one particular server from a game's page. */
export async function playLuduvoServer(place: LuduvoPlace, serverId: string) {
  if (!luduvo.user) {
    toast("Sign in to Luduvo first, then join again.", "info");
    return;
  }
  if (luduvo.playing) {
    toast(`You're already playing ${luduvo.playing.title}. Close it first.`, "info");
    return;
  }
  luduvo.starting = place.id;
  try {
    luduvo.playing = await api.luduvoPlayServer(serverId, place.id, place.title);
    toast(`Joining a ${place.title} server.`, "ok");
  } catch (e) {
    toast(toAppError(e).message, "danger");
  } finally {
    setTimeout(() => (luduvo.starting = null), 4000);
  }
}

/** Joins a private server from a pasted code. Returns whether it started. */
export async function playLuduvoPrivate(place: LuduvoPlace, code: string): Promise<boolean> {
  if (!luduvo.user) {
    toast("Sign in to Luduvo first.", "info");
    return false;
  }
  if (luduvo.playing) {
    toast(`You're already playing ${luduvo.playing.title}. Close it first.`, "info");
    return false;
  }
  luduvo.starting = place.id;
  try {
    luduvo.playing = await api.luduvoPlayPrivate(code, place.id, place.title);
    toast(`Joining a private ${place.title} server.`, "ok");
    return true;
  } catch (e) {
    toast(toAppError(e).message, "danger");
    return false;
  } finally {
    setTimeout(() => (luduvo.starting = null), 4000);
  }
}

/** What a starting Luduvo game is doing, in words; null once it's running. */
export function startingText(r: LuduvoRunning | null): string | null {
  if (!r || r.stage === "playing") return null;
  if (r.stage === "updating") return "Updating Luduvo…";
  if (r.stage === "content") return r.done != null && r.total ? `Downloading game content · ${r.done} of ${r.total}` : "Downloading game content…";
  if (r.stage === "starting") return "Starting the game…";
  return "Checking for updates…";
}

export async function stopLuduvo() {
  try {
    await api.luduvoStop();
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
}

export async function openStudio() {
  try {
    await api.luduvoOpenStudio();
    toast("Opening Luduvo Studio.", "ok");
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
}
