// Playtime Vodka has counted for each game, Steam-style: total, last two weeks and when you last played.
import { api } from "./api";
import type { GameId, Played } from "./types";

export const playtime = $state({ list: [] as Played[], loaded: false });

let started = false;
/** Reads the list now, and again every minute while Vodka is in front (a running game keeps adding to it). */
export async function loadPlaytime() {
  try {
    playtime.list = await api.playtime();
    playtime.loaded = true;
  } catch {
    // Keep what was there.
  }
  if (started) return;
  started = true;
  setInterval(() => {
    if (document.visibilityState === "visible") void loadPlaytime();
  }, 60_000);
}

export function playedOf(game: GameId, id: number): Played | null {
  return playtime.list.find((p) => p.game === game && p.id === id) ?? null;
}

/** "45 min", "3.7 hours", the way Steam rounds it. */
export function hours(seconds: number): string {
  if (seconds < 60) return "under a minute";
  if (seconds < 3600) return `${Math.round(seconds / 60)} min`;
  const h = seconds / 3600;
  return `${h < 10 ? h.toFixed(1) : Math.round(h).toLocaleString()} hours`;
}

/** "Today", "Yesterday", "3 days ago", or a date. */
export function lastPlayed(unixSeconds: number, now = Date.now()): string {
  const then = new Date(unixSeconds * 1000);
  const day = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((day(new Date(now)) - day(then)) / 86_400_000);
  if (days <= 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days} days ago`;
  return then.toLocaleDateString(undefined, { day: "numeric", month: "short", year: then.getFullYear() === new Date(now).getFullYear() ? undefined : "numeric" });
}
