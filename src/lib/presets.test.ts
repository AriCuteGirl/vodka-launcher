import { describe, expect, it } from "vitest";
import { applyPreset, parseFlagJson, PRESETS } from "./presets";
import { NO_OPTIONS } from "./types";

describe("applyPreset", () => {
  it("adds a preset's flags without dropping existing ones", () => {
    const start = { ...NO_OPTIONS, vsync: "off", fflags: { FFlagMine: "True" } };
    const out = applyPreset(start, PRESETS.find((p) => p.id === "lighter")!);
    expect(out.fflags.FFlagMine).toBe("True");
    expect(out.fflags.FFlagDisablePostFx).toBe("True");
    expect(out.vsync).toBe("off");
  });

  it("sets options for FPS presets and leaves flags alone", () => {
    const out = applyPreset({ ...NO_OPTIONS }, PRESETS.find((p) => p.id === "uncapped")!);
    expect(out.fps).toBe("unlimited");
    expect(out.fflags).toEqual({});
  });

  it("never ships the flags Vodka manages itself", () => {
    for (const p of PRESETS) expect(Object.keys(p.fflags ?? {})).not.toContain("DFIntTaskSchedulerTargetFps");
  });
});

describe("parseFlagJson", () => {
  it("accepts an object of simple values", () => {
    expect(parseFlagJson('{"FFlagA": true, "FIntB": 3, "FStringC": "x"}')).toEqual({ flags: { FFlagA: "true", FIntB: "3", FStringC: "x" } });
  });

  it("explains bad input", () => {
    expect(parseFlagJson("{nope")).toHaveProperty("error");
    expect(parseFlagJson("[1,2]")).toHaveProperty("error");
    expect(parseFlagJson('{"FFlagA": {"x": 1}}')).toHaveProperty("error");
  });
});
