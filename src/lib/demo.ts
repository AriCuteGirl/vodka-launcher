// Browser-only stand-in for the Rust backend, used with `pnpm dev` to design and
// screenshot every state without Tauri. Pick a scenario with ?demo=<name>.
import { NO_OPTIONS, type LuduvoFriend, type LuduvoPlace, type LuduvoRunning, type LuduvoUser, type Account, type AppError, type ChatContact, type ChatMessage, type GameStanding, type Item, type Party, type PartyEvent, type Person, type ExitEvent, type Game, type Instance, type LaunchOptions, type MocktailStatus, type Settings, type User } from "./types";

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

const LUDUVO_THUMB = "https://assets.luduvo.com/place-thumbnails/";
const luduvoPlace = (id: number, title: string, creator: string, playing: number, maxPlayers: number, visits: number, icon: string | null): LuduvoPlace => ({
  id, title, creator, description: "", iconUrl: icon ? LUDUVO_THUMB + icon : null, bannerUrl: null, playing, maxPlayers, visits, likes: Math.round(visits / 30), dislikes: 0, playtime: visits * 9,
});
export const DEMO_LUDUVO: LuduvoPlace[] = [
  luduvoPlace(30, "snow battle", "SAD", 0, 20, 188, "589410eb-4799-44e4-a10f-f429e5fff173.png"),
  luduvoPlace(2811, "11/14/26", "Jashluk", 0, 50, 94, "ff9c1dc8-b962-4eb8-a069-d3fcdd4ffd5a.png"),
  luduvoPlace(2869, "BoomBox!", "3xs", 6, 24, 43, "c98361ef-4dbe-4530-a30c-b3fc699fb8e8.png"),
  luduvoPlace(1988, "Murder Mayhem", "foren", 4, 12, 1320, null),
  luduvoPlace(412, "Obby Tower", "brickwise", 2, 30, 870, null),
  luduvoPlace(77, "Hangout Café", "mellow", 1, 40, 2210, null),
];

const ERRORS: Record<string, AppError> = {
  ratelimited: { kind: "rate_limited", message: "Roblox is limiting requests right now. Give it a minute and retry." },
  offline: { kind: "offline", message: "Can't reach Roblox. Check your internet connection." },
  error: { kind: "api", message: "Roblox sent a response Vodka didn't understand (HTTP 500)." },
  expired: { kind: "session_expired", message: "Your Roblox session has expired. Sign in again." },
};

const scenario = () => new URLSearchParams(location.search).get("demo") ?? "default";
let luduvoRunning: LuduvoRunning | null = null;
const luduvoFriend = (id: number, displayName: string, state: string, placeTitle: string | null = null): LuduvoFriend => ({
  id, name: displayName.toLowerCase(), displayName, avatarUrl: null, state, placeId: placeTitle ? 2869 : null, placeTitle, joinable: state === "in_game",
});
let luduvoFriends: LuduvoFriend[] = [luduvoFriend(3, "Nikhil", "in_game", "Stamper Build"), luduvoFriend(4, "poptart", "online"), luduvoFriend(5, "alt9999990929", "offline")];
let luduvoRequests: LuduvoFriend[] = [luduvoFriend(6, "snowmelt", "offline")];
let luduvoCoins = 40;
let demoGroup = { id: 4372130, name: "Gamer Robot Inc", members: 19707257, verified: true, publicEntry: true, myRole: null as string | null };
let demoVipServers = [{ id: 77, name: "Farm room", active: true, expires: "2026-12-14T00:00:00Z", willRenew: true, price: 100, friendsAllowed: false }];
let luduvoVote = { likes: 4, dislikes: 0, vote: 0 };
let luduvoClaimed = false;
let luduvoUser: LuduvoUser | null = scenario() === "signedout" ? null : { id: 9001, name: "puppygirl", displayName: "Puppy Girl", balance: 40 };
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
    engine: scenario() === "windows" ? "roblox" : "mocktail",
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
const item = (id: number, name: string, kind: string, price: number | null = null): Item => ({
  id, name, kind, imageUrl: null, price, creator: price == null ? null : "Frost UGC", description: price == null ? null : "A demo item.", accessory: kind === "Hat" || kind.endsWith("Accessory"),
});
let demoWorn: Item[] = [item(11, "Snow Cap", "Hat"), item(12, "Ice Shades", "FaceAccessory"), item(13, "Black Hoodie", "Shirt"), item(14, "Jeans", "Pants"), item(15, "Torso", "Torso")];
const demoOwned: Item[] = [item(11, "Snow Cap", "Hat"), item(12, "Ice Shades", "FaceAccessory"), item(21, "Cat Ears", "HairAccessory"), item(22, "Frost Scarf", "NeckAccessory"), item(23, "Wings", "BackAccessory")];
let demoRobux = 240;
const demoBought = new Set<number>();
const demoOffer = (id: number) => {
  const price = 40 + (id - 100) * 15;
  const owned = demoBought.has(id);
  const whyNot = owned ? "You already own this." : demoRobux < price ? "You don't have enough Robux for this." : null;
  return { assetId: id, name: "Item", price, buyable: !whyNot, whyNot, owned, balance: demoRobux };
};
const demoStanding: Record<string, GameStanding> = {};
const standing = (id: unknown) => (demoStanding[String(id)] ??= { favorited: false, vote: null, canVote: Number(id) % 2 === 0, whyNot: Number(id) % 2 === 0 ? null : "Play this game first, then you can rate it.", upVotes: 10984750, downVotes: 746771 });
let demoParty: Party | null = null;
let demoPartyEvents: PartyEvent[] = [];
let partyEventId = 0;
const partyEvent = (sender: number | null, kind: PartyEvent["kind"], body: string, payload: PartyEvent["payload"] = null) =>
  demoPartyEvents.push({ id: ++partyEventId, sender, kind, body, payload, createdAt: new Date().toISOString() });
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
  const event: ExitEvent = { id: inst.id, game: inst.game, accountName: inst.accountName, report: { code: outcome === "crashed" ? 1 : 0, outcome, detail }, log: inst.log, restarting, rejoin: null };
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
    case "regroup":
      await wait(500);
      return { moved: instances.filter((i) => i.game.universeId === args.universeId).length, playing: 1, maxPlayers: 40 };
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
    case "party": {
      chatGuard();
      const a = (args.args ?? {}) as Record<string, unknown>;
      const meContact = { robloxId: 1, name: "frostbyte", displayName: "Frostbyte", online: true };
      switch (args.action) {
        case "sync":
          return { party: demoParty, invites: demoParty ? [] : [{ partyId: 9, from: "Glacier" }], events: demoPartyEvents.filter((e) => e.id > ((a.since as number) ?? 0)) };
        case "create":
          demoParty = { id: 1, leader: 1, members: [meContact], pending: [] };
          demoPartyEvents = [];
          partyEvent(null, "system", "Frostbyte started the party.");
          return { ok: true };
        case "accept":
          demoParty = { id: 9, leader: 301, members: [chatContacts[0], meContact], pending: [] };
          demoPartyEvents = [];
          partyEvent(null, "system", "Frostbyte joined.");
          partyEvent(301, "text", "nice, hopping into RIVALS in a sec");
          return { ok: true };
        case "decline":
          return { ok: true };
        case "invite":
          if (demoParty) {
            const c = chatContacts.find((x) => x.robloxId === a.robloxId);
            if (c) {
              demoParty = { ...demoParty, members: [...demoParty.members, c] };
              partyEvent(null, "system", `${c.displayName} joined.`);
            }
          }
          return { ok: true };
        case "kick":
          if (demoParty) demoParty = { ...demoParty, members: demoParty.members.filter((m) => m.robloxId !== a.robloxId) };
          return { ok: true };
        case "send":
          partyEvent(1, "text", String(a.body).trim());
          return { ok: true };
        case "launch":
          if (!instances.some((i) => i.accountId === active()?.id)) throw { kind: "chat", message: "Start a game first, then invite people to it." };
          partyEvent(1, "launch", "", { game: DEMO_GAMES[0], placeId: 17625359962, gameId: "0f0e-11" });
          return { ok: true };
        case "leave":
          demoParty = null;
          return { ok: true };
      }
      throw { kind: "chat", message: "Vodka doesn't know that party action." };
    }
    case "chat_sign_out":
    case "chat_delete_account":
      chatOn = false;
      return null;
    case "game_standing":
      await wait(250);
      return { ...standing(args.universeId) };
    case "set_favorite":
      standing(args.universeId).favorited = args.favorited as boolean;
      return { ...standing(args.universeId) };
    case "set_vote":
      standing(args.universeId).vote = args.like as boolean;
      return { ...standing(args.universeId) };
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
    case "robux_balance":
      return demoRobux;
    case "item_offer":
      await wait(350);
      return demoOffer(args.assetId as number);
    case "buy_item": {
      await wait(700);
      const offer = demoOffer(args.assetId as number);
      if (!offer.buyable) throw { kind: "other", message: offer.whyNot };
      demoRobux -= offer.price;
      demoBought.add(args.assetId as number);
      return demoOffer(args.assetId as number);
    }
    case "avatar_wearing":
      await wait(250);
      return demoWorn;
    case "avatar_inventory":
      await wait(300);
      return { items: args.tab === "accessories" ? demoOwned : [item(13, "Black Hoodie", "Shirt"), item(31, "Pink Hoodie", "Shirt")], next: null };
    case "avatar_set_items": {
      const ids = args.assetIds as number[];
      const all = [...demoWorn, ...demoOwned, item(31, "Pink Hoodie", "Shirt")];
      demoWorn = ids.map((id) => all.find((x) => x.id === id)).filter((x): x is Item => !!x);
      return ids.filter((id) => !all.some((x) => x.id === id));
    }
    case "avatar_set_accessories": {
      const ids = args.assetIds as number[];
      const known = ids.map((id) => demoOwned.find((x) => x.id === id)).filter((x): x is Item => !!x);
      demoWorn = [...demoWorn.filter((x) => !x.accessory), ...known];
      return ids.filter((id) => !demoOwned.some((x) => x.id === id));
    }
    case "avatar_save_outfit":
      await wait(400);
      return null;
    case "catalog_search": {
      await wait(400);
      if (s in ERRORS) throw ERRORS[s];
      const names = ["Cat Ears", "Frost Crown", "Ice Wings", "Snow Scarf", "Glacier Shades", "Polar Tail", "Winter Beanie", "Crystal Horns"];
      const q = String(args.keyword).toLowerCase();
      const found = names.filter((n) => n.toLowerCase().includes(q)).map((n, i) => item(100 + i, n, "Asset", 40 + i * 15));
      return { items: s === "empty" ? [] : found, next: null };
    }
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
    case "luduvo_status":
      await wait(120);
      return { installed: s !== "noluduvo", version: "54", supported: true, installCommand: "curl -fsSL https://assets.luduvo.com/install.sh | sh" };
    case "luduvo_account":
      await wait(150);
      return luduvoUser;
    case "luduvo_sign_in":
      await wait(1500);
      luduvoUser = { id: 9001, name: "puppygirl", displayName: "Puppy Girl", balance: 40 };
      return luduvoUser;
    case "luduvo_cancel_sign_in":
      return null;
    case "luduvo_sign_out":
      luduvoUser = null;
      return null;
    case "luduvo_discover":
      await wait(400);
      if (s in ERRORS) throw ERRORS[s];
      return [
        { id: "recommended", title: "Recommended For You", subtitle: "Curated for your play style", places: DEMO_LUDUVO.slice(0, 5) },
        { id: "active", title: "Most Active", subtitle: "Where everyone is right now", places: [...DEMO_LUDUVO].sort((a, b) => b.playing - a.playing) },
      ];
    case "luduvo_search": {
      await wait(300);
      const q = String(args.query ?? "").toLowerCase();
      return DEMO_LUDUVO.filter((p) => p.title.toLowerCase().includes(q));
    }
    case "luduvo_recent":
      await wait(250);
      return DEMO_LUDUVO.slice(2, 5);
    case "luduvo_play": {
      await wait(700);
      const place = DEMO_LUDUVO.find((p) => p.id === args.placeId);
      luduvoRunning = { placeId: args.placeId as number, title: place?.title ?? "Game", pid: 4242, startedAt: Math.floor(Date.now() / 1000) };
      return luduvoRunning;
    }
    case "open_private_server_page":
      return null;
    case "roblox_group":
      await wait(200);
      return demoGroup;
    case "join_roblox_group":
      await wait(500);
      demoGroup = { ...demoGroup, myRole: "Member" };
      return demoGroup;
    case "open_group_page":
      return null;
    case "private_server_offer":
      await wait(300);
      return { canCreate: true, price: 39 };
    case "create_private_server":
      await wait(600);
      demoVipServers = [...demoVipServers, { id: 90 + demoVipServers.length, name: String(args.name), active: true, expires: "2026-12-16T00:00:00Z", willRenew: true, price: Number(args.agreedPrice), friendsAllowed: Boolean(args.friendsAllowed) }];
      return null;
    case "set_server_friends":
      await wait(300);
      demoVipServers = demoVipServers.map((o) => (o.id === args.serverId ? { ...o, friendsAllowed: Boolean(args.allowed) } : o));
      return null;
    case "owned_servers":
      await wait(250);
      return demoVipServers;
    case "set_server_active":
      demoVipServers = demoVipServers.map((o) => (o.id === args.serverId ? { ...o, active: Boolean(args.active) } : o));
      return null;
    case "server_invite_link":
      return "https://www.roblox.com/share?code=demo1234&type=Server";
    case "game_details": {
      await wait(250);
      const g = DEMO_GAMES.find((x) => x.universeId === args.universeId);
      return {
        universeId: args.universeId,
        name: g?.name ?? "Demo game",
        rootPlaceId: g?.placeId ?? 1,
        creatorId: 4372130,
        creatorType: "Group",
        iconUrl: g?.iconUrl ?? null,
        gallery: [],
        description: `Welcome to ${g?.name ?? "this game"}! Team up with friends, unlock new gear every week and climb the leaderboards.\n\nUpdate log:\n- New map\n- Bug fixes and better performance\n- Halloween event until November 20`,
        creator: "Demo Studios",
        creatorVerified: true,
        playing: g?.playing ?? 1200,
        visits: 1_234_567_890,
        favorites: 4_200_000,
        maxPlayers: 16,
        genre: "Adventure",
        updated: "2026-11-14T18:22:45Z",
        bannerUrl: null,
      };
    }
    case "luduvo_details": {
      await wait(250);
      const place = DEMO_LUDUVO.find((p) => p.id === args.placeId) ?? DEMO_LUDUVO[0];
      return { place: { ...place, description: "its winter time!!! go battle it out and show who's the best out in snow field!\n\nupdate 1.1\n- spawn protection." }, createdAt: 1782496622, updatedAt: 1794441417, ageRating: "all_ages", contentWarnings: ["online interactions"], groupId: 9, groupName: "Snow Builders" };
    }
    case "luduvo_rating":
      return luduvoVote;
    case "luduvo_rate": {
      const v = Number(args.value);
      const was = luduvoVote.vote;
      luduvoVote = {
        likes: luduvoVote.likes + (v === 1 ? 1 : 0) - (was === 1 ? 1 : 0),
        dislikes: luduvoVote.dislikes + (v === -1 ? 1 : 0) - (was === -1 ? 1 : 0),
        vote: v,
      };
      return luduvoVote;
    }
    case "luduvo_play_private":
      await wait(600);
      if (String(args.code).trim().length < 4) throw { kind: "other", message: "That doesn't look like a Luduvo private server code." };
      luduvoRunning = { placeId: args.placeId as number, title: String(args.title), pid: 4245, startedAt: Math.floor(Date.now() / 1000) };
      return luduvoRunning;
    case "luduvo_join_group":
      await wait(300);
      return null;
    case "open_external":
      return null;
    case "luduvo_servers":
      await wait(300);
      return [
        { id: "8513af4f-3721-43ed-97ff-2479392b7f5c", playing: 2, maxPlayers: 16, players: [{ id: 1, name: "EyeLashGaming", avatarUrl: null }, { id: 2, name: "advertiser", avatarUrl: null }] },
        { id: "1d0c55aa-0000-4c2e-9f00-2479392b7f5d", playing: 9, maxPlayers: 16, players: Array.from({ length: 9 }, (_, i) => ({ id: 10 + i, name: `player${i}`, avatarUrl: null })) },
      ];
    case "luduvo_play_server":
      await wait(700);
      luduvoRunning = { placeId: args.placeId as number, title: String(args.title), pid: 4244, startedAt: Math.floor(Date.now() / 1000) };
      return luduvoRunning;
    case "luduvo_friends":
      await wait(300);
      return luduvoFriends;
    case "luduvo_friend_requests":
      return luduvoRequests;
    case "luduvo_answer_request": {
      const who = luduvoRequests.find((r) => r.id === args.userId);
      luduvoRequests = luduvoRequests.filter((r) => r.id !== args.userId);
      if (who && args.accept) luduvoFriends = [...luduvoFriends, who];
      return null;
    }
    case "luduvo_join_friend":
      await wait(700);
      luduvoRunning = { placeId: (args.placeId as number) ?? 0, title: String(args.title), pid: 4243, startedAt: Math.floor(Date.now() / 1000) };
      return luduvoRunning;
    case "luduvo_balance":
      return luduvoCoins;
    case "luduvo_claim_daily":
      await wait(400);
      if (luduvoClaimed) throw { kind: "other", message: "Luduvo: You already claimed today's reward." };
      luduvoClaimed = true;
      luduvoCoins += 10;
      return { amount: 10, balance: luduvoCoins };
    case "luduvo_running":
      return luduvoRunning;
    case "luduvo_stop":
      luduvoRunning = null;
      return null;
    case "luduvo_open_studio":
      return null;
  }
  throw { kind: "other", message: `Unknown command ${cmd}` };
}
