import { describe, expect, it } from "vitest";
import { isRunningMarker } from "./state.svelte";

describe("isRunningMarker", () => {
  it("spots the line Mocktail prints once frames are on screen", () => {
    expect(isRunningMarker("  [perf] present_sample frame=120 fps=60")).toBe(true);
    expect(isRunningMarker("  [window] Window ready; waiting for Roblox frames")).toBe(false);
  });
});
