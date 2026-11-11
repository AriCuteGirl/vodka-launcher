// Browser-only stand-in for the Rust backend, used with `pnpm dev` to design and
// screenshot every state without Tauri. Pick a scenario with ?demo=<name>.
import type { AppError, Game, MocktailStatus, Settings, User } from "./types";

export const DEMO_GAMES: Game[] = [
  { universeId: 6035872082, placeId: 17625359962, name: "RIVALS", playing: 149534, iconUrl: "https://tr.rbxcdn.com/180DAY-1912e777e949b6490e59fa4ac83b1063/256/256/Image/Png/noFilter" },
  { universeId: 994732206, placeId: 2753915549, name: "[🎃] Blox Fruits", playing: 280196, iconUrl: "https://tr.rbxcdn.com/180DAY-8c9483d381a2da30608e6b1bce490bef/256/256/Image/Png/noFilter" },
  { universeId: 383310974, placeId: 920587237, name: "[⚡] Adopt Me!", playing: 160484, iconUrl: "https://tr.rbxcdn.com/180DAY-16972705ca6b7ee982e76269d5b887e8/256/256/Image/Png/noFilter" },
  { universeId: 1686885941, placeId: 4924922222, name: "Brookhaven 🏡RP", playing: 413489, iconUrl: "https://tr.rbxcdn.com/180DAY-59e7f600e77655f8507846254b461a8b/256/256/Image/Png/noFilter" },
  { universeId: 245662005, placeId: 606849621, name: "Jailbreak 🍁🍁", playing: 5766, iconUrl: "https://tr.rbxcdn.com/180DAY-322a1bfda9a8c689eda7d49921f6f0a6/256/256/Image/Png/noFilter" },
  { universeId: 66654135, placeId: 142823291, name: "Murder Mystery 2", playing: 498002, iconUrl: "https://tr.rbxcdn.com/180DAY-ab1484ff8063476d413da09dd25df93f/256/256/Image/Png/noFilter" },
  { universeId: 4777817887, placeId: 13772394625, name: "Blade Ball", playing: 17819, iconUrl: "https://tr.rbxcdn.com/180DAY-38e75c9ff1051ad6ca868f39be2cf095/256/256/Image/Png/noFilter" },
  { universeId: 3317771874, placeId: 8737899170, name: "Pet Simulator 99!", playing: 45021, iconUrl: "https://tr.rbxcdn.com/180DAY-90f12c7dfe5fe57a391edf5e44bb72c5/256/256/Image/Png/noFilter" },
  { universeId: 5750914919, placeId: 16732694052, name: "Fisch 🦈", playing: 65509, iconUrl: "https://tr.rbxcdn.com/180DAY-54c595481b689170c73c9993a83feed3/256/256/Image/Png/noFilter" },
  { universeId: 7018190066, placeId: 116495829188952, name: "Dead Rails", playing: 18859, iconUrl: null },
];

const USER: User = { id: 1, name: "frostbyte", displayName: "Frostbyte", avatarUrl: null };

const ERRORS: Record<string, AppError> = {
  ratelimited: { kind: "rate_limited", message: "Roblox is limiting requests right now. Give it a minute and retry." },
  offline: { kind: "offline", message: "Can't reach Roblox. Check your internet connection." },
  error: { kind: "api", message: "Roblox sent a response Vodka didn't understand (HTTP 500)." },
  expired: { kind: "session_expired", message: "Your Roblox session has expired. Sign in again." },
};

const scenario = () => new URLSearchParams(location.search).get("demo") ?? "default";
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
let settings: Settings = { onboarded: true, shareSessionWithMocktail: false };
let signedIn = !["onboarding", "signedout"].includes(scenario());

function mocktail(): MocktailStatus {
  const missing = scenario() === "nomocktail" || scenario() === "noflatpak";
  return {
    flatpakAvailable: scenario() !== "noflatpak",
    installed: !missing,
    version: missing ? null : "1.0.5",
    robloxVersion: missing ? null : "2.742.1059",
    running: scenario() === "running",
    distro: "Arch Linux",
    installSteps: missing
      ? [
          ...(scenario() === "noflatpak"
            ? [
                { label: "Install Flatpak", command: "sudo pacman -S --needed flatpak" },
                { label: "Add the Flathub store", command: "flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo" },
              ]
            : []),
          { label: "Install Mocktail", command: "flatpak install --user flathub space.bigrat.mocktail" },
        ]
      : [],
  };
}

async function games(list: Game[]): Promise<Game[]> {
  const s = scenario();
  if (s === "loading") await new Promise(() => {});
  await wait(350);
  if (s in ERRORS) throw ERRORS[s];
  return s === "empty" ? [] : list;
}

type Listener = (event: { payload: unknown }) => void;
const listeners = new Map<string, Set<Listener>>();
const emit = (name: string, payload: unknown) => listeners.get(name)?.forEach((l) => l({ payload }));

export function demoListen(name: string, handler: Listener): () => void {
  if (!listeners.has(name)) listeners.set(name, new Set());
  listeners.get(name)!.add(handler);
  return () => listeners.get(name)?.delete(handler);
}

export async function demoInvoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  const s = scenario();
  switch (cmd) {
    case "mocktail_status":
      await wait(200);
      return mocktail();
    case "current_account":
      await wait(200);
      return signedIn ? USER : null;
    case "continue_playing":
      return games(DEMO_GAMES.slice(0, 6));
    case "favorites":
      return games(DEMO_GAMES.slice(3));
    case "get_settings":
      return { ...settings, onboarded: s === "onboarding" ? false : settings.onboarded };
    case "save_settings":
      settings = args.settings as Settings;
      return null;
    case "begin_login":
      setTimeout(() => {
        signedIn = true;
        emit("auth://signed-in", USER);
      }, 900);
      return null;
    case "sign_out":
      signedIn = false;
      return null;
    case "session_log":
      return [];
    case "launch_game": {
      if (!mocktail().installed) throw { kind: "mocktail_missing", message: "Mocktail is not installed." };
      const lines = ["  [runtime] loaded 0 FFlag overrides", "  [update] payload 2.742.1059 verified", "  [window] direct-vulkan surface ready", "  [perf] present_sample frame=1"];
      lines.forEach((l, i) => setTimeout(() => emit("mocktail://log", l), 300 * (i + 1)));
      if (s === "crash") {
        setTimeout(() => emit("mocktail://log", "[FATAL] Vulkan device lost"), 1500);
        setTimeout(() => emit("mocktail://exit", { code: 1, outcome: "crashed", detail: "Vulkan device lost" }), 1700);
      } else if (s === "running") {
        setTimeout(() => emit("mocktail://exit", { code: 0, outcome: "forwarded", detail: null }), 800);
      }
      return null;
    }
  }
  throw { kind: "other", message: `Unknown command ${cmd}` };
}
