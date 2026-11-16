// Spots friends coming online or starting a game between two refreshes of a friends list.

export type Activity = "offline" | "online" | "playing";

export interface Seen {
  id: number;
  activity: Activity;
  /** The game they're in, when they're playing. */
  game: string | null;
}

export interface FriendChange {
  id: number;
  kind: "online" | "playing";
  game: string | null;
}

/**
 * What changed from `before` to `now`. Coming online from offline is "online"; starting a game, or switching to
 * another one, is "playing". Leaving a game or going offline isn't worth a pop-up. A friend not seen before (just
 * added, or the first refresh) never counts: the first list is only a baseline.
 */
export function friendChanges(before: Map<number, Seen>, now: Seen[]): FriendChange[] {
  const out: FriendChange[] = [];
  for (const f of now) {
    const was = before.get(f.id);
    if (!was) continue;
    if (f.activity === "playing" && (was.activity !== "playing" || (f.game && was.game && f.game !== was.game))) {
      out.push({ id: f.id, kind: "playing", game: f.game });
    } else if (f.activity === "online" && was.activity === "offline") {
      out.push({ id: f.id, kind: "online", game: null });
    }
  }
  return out;
}
