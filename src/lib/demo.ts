// Browser-only stand-in for the Rust backend, used with `pnpm dev` to design and
// screenshot every state without Tauri. Pick a scenario with ?demo=<name>.
import { NO_OPTIONS, type Account, type AppError, type ChatContact, type ChatMessage, type Person, type ExitEvent, type Game, type Instance, type LaunchOptions, type MocktailStatus, type Settings, type User } from "./types";

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

const ERRORS: Record<string, AppError> = {
  ratelimited: { kind: "rate_limited", message: "Roblox is limiting requests right now. Give it a minute and retry." },
  offline: { kind: "offline", message: "Can't reach Roblox. Check your internet connection." },
  error: { kind: "api", message: "Roblox sent a response Vodka didn't understand (HTTP 500)." },
  expired: { kind: "session_expired", message: "Your Roblox session has expired. Sign in again." },
};

const scenario = () => new URLSearchParams(location.search).get("demo") ?? "default";
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const signedOut = ["onboarding", "signedout"].includes(scenario());

let accounts: Account[] = signedOut
  ? []
  : [
      { id: 1, name: "frostbyte", displayName: "Frostbyte", avatarUrl: null, active: true, presence: { kind: "online", location: "Website" }, inVodkaGame: false, clientVersion: "2.742.1059" },
      { id: 2, name: "snowmelt_alt", displayName: "Snowmelt", avatarUrl: null, active: false, presence: { kind: "offline", location: null }, inVodkaGame: false, clientVersion: null },
    ];
let settings: Settings = { onboarded: true, activeAccount: signedOut ? null : 1, multiInstanceAccepted: false, defaults: { ...NO_OPTIONS }, accountOptions: {} };
let instances: Instance[] = [];
let nextId = 1;

/** What the demo "measures" for an account, from its saved FPS option. */
function demoFps(accountId: number): number {
  const fps = settings.accountOptions[String(accountId)]?.fps ?? settings.defaults.fps;
  const target = fps === "unlimited" ? 240 : fps === "display" || fps == null ? 60 : Number(fps);
  return target - Math.random() * 1.5;
}

const active = () => accounts.find((a) => a.active) ?? null;
const asUser = (a: Account): User => ({ id: a.id, name: a.name, displayName: a.displayName, avatarUrl: a.avatarUrl });

function mocktail(): MocktailStatus {
  const missing = scenario() === "nomocktail" || scenario() === "noflatpak";
  return {
    flatpakAvailable: scenario() !== "noflatpak",
    installed: !missing,
    version: missing ? null : "1.0.5",
    robloxVersion: missing ? null : "2.742.1059",
    running: instances.length > 0,
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
  if (s === "empty") return [];
  // The second account has different games, so switching is visible.
  return active()?.id === 2 ? [...list].reverse().slice(0, 4) : list;
}

const person = (id: number, displayName: string, kind: Person["presence"]["kind"], location: string | null = null, joinable = false): Person => ({
  id,
  name: displayName.toLowerCase().replace(/\W/g, "_"),
  displayName,
  avatarUrl: null,
  presence: { kind, location, placeId: joinable ? 17625359962 : null, universeId: joinable ? 6035872082 : null, gameId: joinable ? "0f0e-11" : null },
});
let demoFriends: Person[] = [
  person(101, "Glacier", "in_game", "RIVALS", true),
  person(102, "Mint Julep", "in_game", "Brookhaven 🏡RP"),
  person(103, "Rime", "online", "Website"),
  person(104, "Sleet", "studio", "Studio"),
  person(105, "Tundra", "offline"),
  person(106, "Aurora", "offline"),
  person(107, "Borealis", "offline"),
];
let demoRequests: Person[] = [person(201, "Hailstone", "offline"), person(202, "Permafrost", "offline")];

let chatOn = scenario() === "chat";
let chatContacts: ChatContact[] = [
  { robloxId: 301, name: "glacier_gg", displayName: "Glacier", online: true },
  { robloxId: 302, name: "rime", displayName: "Rime", online: false },
];
let chatId = 3;
let chatMessages: ChatMessage[] = [
  { id: 1, sender: 301, recipient: 1, kind: "text", body: "yo, you on?", payload: null, createdAt: new Date(Date.now() - 600_000).toISOString() },
  { id: 2, sender: 1, recipient: 301, kind: "text", body: "yeah give me a sec", payload: null, createdAt: new Date(Date.now() - 540_000).toISOString() },
  { id: 3, sender: 301, recipient: 1, kind: "invite", body: "", payload: { game: DEMO_GAMES[0], placeId: 17625359962, gameId: "0f0e-11" }, createdAt: new Date(Date.now() - 60_000).toISOString() },
];
const chatGuard = () => {
  if (!chatOn) throw { kind: "chat_signed_out", message: "You're not signed in to Vodka chat on this account." };
};

type Listener = (event: { payload: unknown }) => void;
const listeners = new Map<string, Set<Listener>>();
const emit = (name: string, payload: unknown) => listeners.get(name)?.forEach((l) => l({ payload }));

export function demoListen(name: string, handler: Listener): () => void {
  if (!listeners.has(name)) listeners.set(name, new Set());
  listeners.get(name)!.add(handler);
  return () => listeners.get(name)?.delete(handler);
}

function exit(inst: Instance, outcome: ExitEvent["report"]["outcome"], detail: string | null, restarting = false) {
  instances = instances.filter((i) => i.id !== inst.id);
  const event: ExitEvent = { id: inst.id, game: inst.game, accountName: inst.accountName, report: { code: outcome === "crashed" ? 1 : 0, outcome, detail }, log: inst.log, restarting };
  emit("instances://exit", event);
  emit("instances://changed", null);
}

function launch(game: Game, account: Account): number {
  if (instances.some((i) => i.accountId === account.id)) {
    throw { kind: "launch", message: `Mocktail could not be started: ${account.displayName} is already in a game. Close it, or switch to another account.` };
  }
  if (instances.length >= 10) throw { kind: "launch", message: "Mocktail could not be started: Vodka runs at most 10 games at once. Close one first." };
  const inst: Instance = {
    id: nextId++,
    slot: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].find((s) => !instances.some((i) => i.slot === s)) ?? 0,
    accountId: account.id,
    accountName: account.displayName,
    game,
    phase: "starting",
    startedAtMs: Date.now(),
    cpuPercent: 0,
    memoryBytes: 180 * 1024 * 1024,
    fps: null,
    log: [],
  };
  instances.push(inst);
  emit("instances://changed", null);
  const lines = ["  [runtime] loaded 0 FFlag overrides", "  [auth] typed Roblox credential ready for native startup", "  [window] Window ready; waiting for Roblox frames"];
  lines.forEach((l, i) => setTimeout(() => inst.log.push(l), 300 * (i + 1)));
  setTimeout(() => {
    if (!instances.includes(inst)) return;
    if (scenario() === "crash") {
      inst.log.push("[FATAL] Vulkan device lost");
      return exit(inst, "crashed", "Vulkan device lost");
    }
    inst.phase = "playing";
    inst.startedAtMs = Date.now();
    inst.log.push("  [perf] present_sample frame=1");
    emit("instances://changed", null);
  }, 1400);
  return inst.id;
}

export async function demoInvoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  const s = scenario();
  switch (cmd) {
    case "mocktail_status":
      await wait(150);
      return mocktail();
    case "current_account": {
      await wait(150);
      const a = active();
      return a ? asUser(a) : null;
    }
    case "list_accounts":
      return accounts.map((a) => ({ ...a, inVodkaGame: instances.some((i) => i.accountId === a.id) }));
    case "switch_account":
      accounts = accounts.map((a) => ({ ...a, active: a.id === args.accountId }));
      settings.activeAccount = args.accountId as number;
      return null;
    case "remove_account":
      if (instances.some((i) => i.accountId === args.accountId)) throw { kind: "other", message: "That account is in a game. Close the game first." };
      accounts = accounts.filter((a) => a.id !== args.accountId);
      if (!active() && accounts[0]) accounts[0].active = true;
      settings.activeAccount = active()?.id ?? null;
      return null;
    case "continue_playing":
      return games(DEMO_GAMES.slice(0, 6));
    case "favorites":
      return games(DEMO_GAMES.slice(3));
    case "search_games": {
      const q = String(args.query).toLowerCase();
      const found = await games(DEMO_GAMES);
      return s === "empty" ? [] : found.filter((g) => g.name.toLowerCase().includes(q));
    }
    case "game_charts": {
      const all = await games(DEMO_GAMES);
      return all.length
        ? [
            { name: "Top Trending", games: all },
            { name: "Top Playing Now", games: [...all].sort((a, b) => (b.playing ?? 0) - (a.playing ?? 0)) },
            { name: "Up-and-Coming", games: [...all].reverse() },
          ]
        : [];
    }
    case "get_settings":
      return { ...settings, onboarded: s === "onboarding" ? false : settings.onboarded };
    case "set_flags":
      settings = { ...settings, ...(args.flags as Partial<Settings>) };
      return settings;
    case "begin_login":
      setTimeout(() => {
        const id = accounts.length ? Math.max(...accounts.map((a) => a.id)) + 1 : 1;
        const added: Account = { id, name: id === 1 ? "frostbyte" : `icicle_${id}`, displayName: id === 1 ? "Frostbyte" : `Icicle ${id}`, avatarUrl: null, active: true, presence: { kind: "online", location: "Website" }, inVodkaGame: false, clientVersion: null };
        accounts = [...accounts.map((a) => ({ ...a, active: false })), added];
        settings.activeAccount = id;
        emit("auth://signed-in", asUser(added));
      }, 900);
      return null;
    case "launch_game": {
      if (!mocktail().installed) throw { kind: "mocktail_missing", message: "Mocktail is not installed." };
      const a = args.accountId == null ? active() : accounts.find((x) => x.id === args.accountId);
      if (!a) throw { kind: "not_signed_in", message: "You're not signed in to Roblox." };
      return launch(args.game as Game, a);
    }
    case "list_servers":
      await wait(400);
      if (s in ERRORS) throw ERRORS[s];
      return {
        public: s === "empty" ? [] : [1, 3, 7, 12, 19, 26, 31, 38].map((playing, i) => ({ id: `0f0e-${i}`, playing, maxPlayers: 40, ping: 60 + i * 17 })),
        private: s === "empty" ? [] : [{ name: "Farm room", owner: "Frostbyte", playing: 2, maxPlayers: 20, accessCode: "3f2a-9c" }],
        privateError: null,
      };
    case "friend_location": {
      const friend = demoFriends.find((f) => f.id === args.userId);
      if (!friend?.presence.gameId) throw { kind: "other", message: "That friend's game can't be joined: their privacy settings hide the server." };
      return { game: DEMO_GAMES[0], placeId: 17625359962, gameId: friend.presence.gameId };
    }
    case "list_friends":
      await wait(300);
      if (s in ERRORS) throw ERRORS[s];
      return s === "empty" || active()?.id === 2 ? [] : demoFriends;
    case "list_friend_requests":
      return s === "empty" || active()?.id === 2 ? [] : demoRequests;
    case "answer_friend_request": {
      const who = demoRequests.find((r) => r.id === args.userId);
      demoRequests = demoRequests.filter((r) => r.id !== args.userId);
      if (who && args.accept) demoFriends = [...demoFriends, who];
      return null;
    }
    case "join_friend": {
      const friend = demoFriends.find((f) => f.id === args.userId);
      const a = active();
      if (!friend?.presence.gameId) throw { kind: "other", message: "That friend's game can't be joined: their privacy settings hide the server." };
      if (!a) throw { kind: "not_signed_in", message: "You're not signed in to Roblox." };
      return launch(DEMO_GAMES[0], a);
    }
    case "chat_enabled":
      return chatOn;
    case "chat_start_verify":
      await wait(300);
      return { challenge: "breeze saffron bridge cloud copper lantern" };
    case "chat_finish_verify":
      await wait(600);
      chatOn = true;
      return { me: { robloxId: 1, name: "frostbyte", displayName: "Frostbyte", code: "VDK-FRST-BY73" } };
    case "chat_sync":
      chatGuard();
      return {
        me: { robloxId: 1, name: "frostbyte", displayName: "Frostbyte", code: "VDK-FRST-BY73" },
        contacts: chatContacts,
        blocked: [],
        messages: chatMessages.filter((m) => m.id > (args.since as number)),
      };
    case "chat_add_contact": {
      chatGuard();
      if (!/^VDK-[A-Z2-9]{4}-[A-Z2-9]{4}$/i.test(String(args.code).trim())) throw { kind: "chat", message: "Vodka codes look like VDK-ABCD-2345." };
      const contact = { robloxId: 300 + chatContacts.length + 5, name: "new_friend", displayName: "New Friend", online: true };
      chatContacts = [...chatContacts, contact];
      return { contact };
    }
    case "chat_remove_contact":
    case "chat_block":
      chatContacts = chatContacts.filter((c) => c.robloxId !== args.userId);
      return { ok: true };
    case "chat_report":
      return { ok: true };
    case "chat_send":
    case "chat_invite": {
      chatGuard();
      if (cmd === "chat_invite" && !instances.some((i) => i.accountId === active()?.id)) throw { kind: "chat", message: "Start a game first, then invite people to it." };
      const message: ChatMessage = {
        id: ++chatId,
        sender: 1,
        recipient: args.to as number,
        kind: cmd === "chat_invite" ? "invite" : "text",
        body: cmd === "chat_invite" ? "" : String(args.body).trim(),
        payload: cmd === "chat_invite" ? { game: DEMO_GAMES[0], placeId: 17625359962, gameId: "0f0e-11" } : null,
        createdAt: new Date().toISOString(),
      };
      chatMessages = [...chatMessages, message];
      return { message };
    }
    case "chat_sign_out":
    case "chat_delete_account":
      chatOn = false;
      return null;
    case "refresh_instance_games":
      return false;
    case "avatar":
      await wait(400);
      if (s in ERRORS) throw ERRORS[s];
      return {
        pictureUrl: null,
        outfits: s === "empty" ? [] : ["Winter fit", "Bacon classic", "Beach day", "All black", "Pastel", "Knight"].map((name, i) => ({ id: i + 1, name, imageUrl: null })),
      };
    case "avatar_picture":
      return null;
    case "wear_outfit":
      await wait(700);
      return [];
    case "headshots":
      return {};
    case "list_instances":
      return instances.map((i) => ({
        ...i,
        cpuPercent: i.phase === "playing" ? 80 + Math.random() * 60 : 15 + Math.random() * 20,
        memoryBytes: (i.phase === "playing" ? 1700 + Math.random() * 60 : 400) * 1024 * 1024,
        fps: i.phase === "playing" ? demoFps(i.accountId) : null,
      }));
    case "check_fflag": {
      const name = String(args.name).trim();
      const value = String(args.value).trim();
      const kind = /^(?:DF|SF|F)(Flag|Int|String|Log)[A-Za-z0-9_]+$/.exec(name)?.[1];
      if (!kind) throw { kind: "other", message: "Flag names start with FFlag, FInt, FString or FLog (optionally DF… or SF…), followed by letters and digits." };
      if (name === "DFIntTaskSchedulerTargetFps") throw { kind: "other", message: "Vodka sets this flag from the FPS option. Change FPS there instead." };
      if (kind === "Flag") {
        if (!/^(true|false)$/i.test(value)) throw { kind: "other", message: `${name} is an on/off flag: use True or False.` };
        return [name, value.toLowerCase() === "true" ? "True" : "False"];
      }
      if (kind !== "String" && !/^-?\d+$/.test(value)) throw { kind: "other", message: `${name} needs a whole number.` };
      return [name, value];
    }
    case "save_launch_options": {
      const options = args.options as LaunchOptions | null;
      if (args.accountId == null) settings.defaults = options ?? { ...NO_OPTIONS };
      else if (options) settings.accountOptions[String(args.accountId)] = options;
      else delete settings.accountOptions[String(args.accountId)];
      return { ...settings };
    }
    case "rollback_client": {
      const a = accounts.find((x) => x.id === args.accountId);
      if (!a?.clientVersion) throw { kind: "other", message: "This account hasn't played yet, so there is nothing to roll back." };
      a.clientVersion = "2.736.1408";
      const key = String(args.accountId);
      settings.accountOptions[key] = { ...(settings.accountOptions[key] ?? NO_OPTIONS), holdUpdates: true };
      return { ...settings };
    }
    case "stop_instance": {
      const inst = instances.find((i) => i.id === args.id);
      if (!inst) throw { kind: "other", message: "That game has already closed." };
      const account = accounts.find((a) => a.id === inst.accountId)!;
      setTimeout(() => {
        exit(inst, "closed", null, args.restart as boolean);
        if (args.restart) launch(inst.game, account);
      }, 500);
      return null;
    }
  }
  throw { kind: "other", message: `Unknown command ${cmd}` };
}
