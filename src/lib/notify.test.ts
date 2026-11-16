import { describe, expect, it } from "vitest";
import { friendChanges, type Seen } from "./notify";

const map = (list: Seen[]) => new Map(list.map((s) => [s.id, s]));
const seen = (id: number, activity: Seen["activity"], game: string | null = null): Seen => ({ id, activity, game });

describe("friendChanges", () => {
  it("notices coming online and starting or switching a game", () => {
    const before = map([seen(1, "offline"), seen(2, "online"), seen(3, "playing", "RIVALS"), seen(4, "offline")]);
    const now = [seen(1, "online"), seen(2, "playing", "Brookhaven"), seen(3, "playing", "Blox Fruits"), seen(4, "playing", "Fisch")];
    expect(friendChanges(before, now)).toEqual([
      { id: 1, kind: "online", game: null },
      { id: 2, kind: "playing", game: "Brookhaven" },
      { id: 3, kind: "playing", game: "Blox Fruits" },
      { id: 4, kind: "playing", game: "Fisch" },
    ]);
  });

  it("stays quiet for leaving, staying put, and people it hasn't seen before", () => {
    const before = map([seen(1, "playing", "RIVALS"), seen(2, "online"), seen(3, "playing", "RIVALS")]);
    const now = [seen(1, "online"), seen(2, "offline"), seen(3, "playing", "RIVALS"), seen(9, "playing", "Fisch")];
    expect(friendChanges(before, now)).toEqual([]);
  });

  it("doesn't count an unknown game name as switching", () => {
    expect(friendChanges(map([seen(1, "playing", null)]), [seen(1, "playing", "RIVALS")])).toEqual([]);
  });
});
