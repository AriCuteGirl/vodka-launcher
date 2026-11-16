import type { LaunchOptions } from "./types";

export interface Preset {
  id: string;
  name: string;
  description: string;
  /** True when Vodka's author measured the effect; false when the flags are only passed on to Roblox. */
  verified: boolean;
  options?: Partial<Omit<LaunchOptions, "fflags">>;
  fflags?: Record<string, string>;
}

export const PRESETS: Preset[] = [
  {
    id: "uncapped",
    name: "Uncapped FPS",
    description: "Removes the frame cap (Mocktail tops out at 240 FPS on Linux).",
    verified: true,
    options: { fps: "unlimited" },
  },
  {
    id: "saver",
    name: "Battery saver",
    description: "Caps the game at 30 FPS to cut heat and power use.",
    verified: true,
    options: { fps: "30" },
  },
  {
    id: "lighter",
    name: "Lighter graphics",
    description: "Lowest quality level, no shadows, no post-processing.",
    verified: false,
    fflags: { DFIntDebugFRMQualityLevelOverride: "1", FIntRenderShadowIntensity: "0", FFlagDisablePostFx: "True" },
  },
  {
    id: "textures",
    name: "Low-res textures",
    description: "Forces the lowest texture quality to save memory.",
    verified: false,
    fflags: { DFFlagTextureQualityOverrideEnabled: "True", DFIntTextureQualityOverride: "0" },
  },
];

/** Returns `options` with the preset applied on top. */
export function applyPreset(options: LaunchOptions, preset: Preset): LaunchOptions {
  return { ...options, ...preset.options, fflags: { ...options.fflags, ...preset.fflags } };
}

/** Parses pasted JSON into flags, or says what is wrong with it. */
export function parseFlagJson(text: string): { flags: Record<string, string> } | { error: string } {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { error: "That isn't valid JSON." };
  }
  if (!data || typeof data !== "object" || Array.isArray(data)) return { error: 'Expected an object like {"FFlagExample": "True"}.' };
  const flags: Record<string, string> = {};
  for (const [name, value] of Object.entries(data)) {
    if (!["string", "number", "boolean"].includes(typeof value)) return { error: `${name} has a value that isn't text, a number or true/false.` };
    flags[name] = String(value);
  }
  return { flags };
}
