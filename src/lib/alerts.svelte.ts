// Friend pop-ups: "Nikhil started playing RIVALS" with Join, for Roblox and Luduvo friends alike.
import { friendChanges, type Activity, type Seen } from "./notify";
import type { GameId } from "./types";

export interface FriendAlert {
  key: number;
  game: GameId;
  name: string;
  avatarUrl: string | null;
  text: string;
  /** Where the profile is. */
  profile: string;
  /** Joins them, when their server can be joined. */
  join: (() => void) | null;
}

const PREFS_KEY = "vodka-friend-alerts";

function readPrefs(): { online: boolean; playing: boolean } {
  try {
    return { online: true, playing: true, ...JSON.parse(localStorage.getItem(PREFS_KEY) ?? "{}") };
  } catch {
    return { online: true, playing: true };
  }
}

export const alerts = $state({
  list: [] as FriendAlert[],
  /** Remembered on this computer; both on unless turned off in Settings. */
  prefs: readPrefs(),
});

export function savePrefs() {
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(alerts.prefs));
  } catch {
    // It just won't be remembered.
  }
}

export function dismissAlert(key: number) {
  alerts.list = alerts.list.filter((a) => a.key !== key);
}

let nextKey = 1;
const MAX_PER_ROUND = 3;
const SHOW_MS = 10_000;

/** The last list seen for each game and account, so a switch of account starts a fresh baseline. */
const lastSeen = new Map<string, Map<number, Seen>>();

export interface FriendNow {
  id: number;
  name: string;
  avatarUrl: string | null;
  activity: Activity;
  game: string | null;
  join: (() => void) | null;
  profile: string;
}

/** Compares a fresh friends list with the last one for this game and account, and pops up what changed. */
export function noteFriends(game: GameId, accountId: number, friends: FriendNow[]) {
  const key = `${game}:${accountId}`;
  const before = lastSeen.get(key);
  lastSeen.set(key, new Map(friends.map((f) => [f.id, { id: f.id, activity: f.activity, game: f.game }])));
  if (!before) return;
  const changes = friendChanges(before, friends).filter((c) => (c.kind === "online" ? alerts.prefs.online : alerts.prefs.playing));
  for (const c of changes.slice(0, MAX_PER_ROUND)) {
    const f = friends.find((x) => x.id === c.id);
    if (!f) continue;
    const alert: FriendAlert = {
      key: nextKey++,
      game,
      name: f.name,
      avatarUrl: f.avatarUrl,
      text: c.kind === "online" ? "is online" : c.game ? `started playing ${c.game}` : "started a game",
      profile: f.profile,
      join: c.kind === "playing" ? f.join : null,
    };
    alerts.list = [...alerts.list.slice(-4), alert];
    setTimeout(() => dismissAlert(alert.key), SHOW_MS);
  }
}
