import { describe, expect, it } from "vitest";
import { compact, elapsed, matches, memory } from "./format";

describe("compact", () => {
  it("keeps small numbers and shortens big ones", () => {
    expect(compact(0)).toBe("0");
    expect(compact(999)).toBe("999");
    expect(compact(1000)).toBe("1K");
    expect(compact(5766)).toBe("5.8K");
    expect(compact(149534)).toBe("150K");
    expect(compact(2_100_000)).toBe("2.1M");
    expect(compact(65_523_923_109)).toBe("65.5B");
  });
});

describe("elapsed", () => {
  it("formats minutes and hours", () => {
    expect(elapsed(5)).toBe("0m 05s");
    expect(elapsed(245)).toBe("4m 05s");
    expect(elapsed(4320)).toBe("1h 12m");
    expect(elapsed(-3)).toBe("0m 00s");
  });
});

describe("memory", () => {
  it("switches to GB for large values", () => {
    expect(memory(512 * 1024 * 1024)).toBe("512 MB");
    expect(memory(1.5 * 1024 * 1024 * 1024)).toBe("1.5 GB");
  });
});

describe("matches", () => {
  it("ignores case, emoji and punctuation", () => {
    expect(matches("[🎃] Blox Fruits", "blox")).toBe(true);
    expect(matches("Brookhaven 🏡RP", "brookhaven rp")).toBe(true);
    expect(matches("RIVALS", "")).toBe(true);
    expect(matches("RIVALS", "fisch")).toBe(false);
  });
});
