export interface User {
  id: number;
  name: string;
  displayName: string;
  avatarUrl: string | null;
}

export interface Game {
  universeId: number;
  placeId: number;
  name: string;
  playing: number | null;
  iconUrl: string | null;
}

export interface Server {
  id: string;
  playing: number;
  maxPlayers: number;
  ping: number | null;
}

export interface PrivateServer {
  name: string;
  owner: string | null;
  playing: number;
  maxPlayers: number;
  accessCode: string;
}

export interface ServerLists {
  public: Server[];
  private: PrivateServer[];
  privateError: string | null;
}

/** Which server to start in. Leave it out to let Roblox choose. */
export type Target = { kind: "server"; placeId: number; gameId: string } | { kind: "private"; accessCode: string };

export interface FriendLocation {
  game: Game;
  placeId: number;
  gameId: string;
}

export interface ChatMe {
  robloxId: number;
  name: string;
  displayName: string;
  /** The code to give people so they can add you. */
  code: string;
}

export interface ChatContact {
  robloxId: number;
  name: string;
  displayName: string;
  online: boolean;
}

export interface ChatInvite {
  game: Game;
  placeId: number;
  /** The exact server, when the sender's game shares it. */
  gameId: string | null;
}

export interface ChatMessage {
  id: number;
  sender: number;
  recipient: number;
  kind: "text" | "invite";
  body: string;
  payload: ChatInvite | null;
  createdAt: string;
}

export interface ChatSync {
  me: ChatMe;
  contacts: ChatContact[];
  blocked: number[];
  messages: ChatMessage[];
}

export interface Party {
  id: number;
  leader: number;
  members: ChatContact[];
  /** User ids with an open invite. */
  pending: number[];
}

export interface PartyEvent {
  id: number;
  /** null for system lines such as "X joined". */
  sender: number | null;
  kind: "text" | "launch" | "system";
  body: string;
  payload: ChatInvite | null;
  createdAt: string;
}

export interface PartyInvite {
  partyId: number;
  from: string;
}

export interface PartySync {
  party: Party | null;
  invites: PartyInvite[];
  events: PartyEvent[];
}

/** An avatar item: worn, owned, or for sale. */
export interface Item {
  id: number;
  name: string;
  /** Roblox's type name ("Hat", "Shirt", …), or "Bundle". */
  kind: string;
  imageUrl: string | null;
  price: number | null;
  creator: string | null;
  description: string | null;
  /** One of the accessory types the Advanced box takes by id. */
  accessory: boolean;
}

export interface ItemPage {
  items: Item[];
  next: string | null;
}

/** What buying an item would involve, before the user confirms. */
export interface Offer {
  assetId: number;
  name: string;
  price: number | null;
  /** Vodka can buy it: on sale at a fixed price, enough Robux, not owned, not a limited. */
  buyable: boolean;
  whyNot: string | null;
  owned: boolean;
  balance: number | null;
}

export interface GameStanding {
  favorited: boolean;
  /** true liked, false disliked, null not rated. */
  vote: boolean | null;
  canVote: boolean;
  whyNot: string | null;
  upVotes: number | null;
  downVotes: number | null;
}

export interface Outfit {
  id: number;
  name: string;
  imageUrl: string | null;
}

export interface AvatarView {
  pictureUrl: string | null;
  outfits: Outfit[];
}

export interface Chart {
  name: string;
  games: Game[];
}

export interface InstallStep {
  label: string;
  command: string;
}

export interface MocktailStatus {
  /** What runs the game: Mocktail on Linux, the official Roblox client on Windows. */
  engine: "mocktail" | "roblox";
  flatpakAvailable: boolean;
  installed: boolean;
  version: string | null;
  robloxVersion: string | null;
  running: boolean;
  distro: string;
  installSteps: InstallStep[];
}

export interface Presence {
  kind: "offline" | "online" | "in_game" | "studio";
  location: string | null;
  placeId?: number | null;
  rootPlaceId?: number | null;
  universeId?: number | null;
  /** Present when the server they're on can be joined. */
  gameId?: string | null;
}

/** A friend, or someone who sent a friend request. */
export interface Person {
  id: number;
  name: string;
  displayName: string;
  avatarUrl: string | null;
  presence: Presence;
}

export interface Account {
  id: number;
  name: string;
  displayName: string;
  avatarUrl: string | null;
  active: boolean;
  presence: Presence | null;
  /** In a game that Vodka started. */
  inVodkaGame: boolean;
  clientVersion: string | null;
}

export interface Instance {
  id: number;
  slot: number;
  accountId: number;
  accountName: string;
  game: Game;
  phase: "starting" | "playing";
  startedAtMs: number;
  cpuPercent: number;
  memoryBytes: number;
  fps: number | null;
  log: string[];
}

export interface ExitEvent {
  id: number;
  game: Game;
  accountName: string;
  report: ExitReport;
  log: string[];
  restarting: boolean;
  /** Why Vodka is bringing the game back by itself, or "limit" when it stopped trying. */
  rejoin: "crash" | "disconnect" | "limit" | "signin" | "signin_limit" | null;
}

/** `null` means "don't override": the account falls back to the defaults, the defaults to Mocktail's own config. */
export interface LaunchOptions {
  fps: string | null;
  backend: string | null;
  vsync: string | null;
  presentMode: string | null;
  multithreadedRendering: boolean | null;
  gamemode: string | null;
  /** Windows: the graphics API Roblox should prefer. */
  renderer: string | null;
  memoryLimitMb: number | null;
  holdUpdates: boolean | null;
  /** Start the game again by itself after a crash or disconnect. */
  autoRejoin: boolean | null;
  fflags: Record<string, string>;
}

export const NO_OPTIONS: LaunchOptions = {
  fps: null,
  renderer: null,
  backend: null,
  vsync: null,
  presentMode: null,
  multithreadedRendering: null,
  gamemode: null,
  memoryLimitMb: null,
  holdUpdates: null,
  autoRejoin: null,
  fflags: {},
};

export interface Settings {
  onboarded: boolean;
  activeAccount: number | null;
  multiInstanceAccepted: boolean;
  defaults: LaunchOptions;
  accountOptions: Record<string, LaunchOptions>;
}

export interface ExitReport {
  code: number | null;
  outcome: "closed" | "forwarded" | "crashed";
  detail: string | null;
}

export type ErrorKind =
  | "flatpak_missing"
  | "mocktail_missing"
  | "launch"
  | "not_signed_in"
  | "session_expired"
  | "rate_limited"
  | "offline"
  | "roblox_down"
  | "api"
  | "keyring"
  | "blocked_url"
  | "chat"
  | "chat_signed_out"
  | "other";

export interface AppError {
  kind: ErrorKind;
  message: string;
}

/** Whatever was thrown, as an error the UI can show. */
export function toAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "kind" in e && "message" in e) return e as AppError;
  return { kind: "other", message: e instanceof Error ? e.message : String(e) };
}

export type Loadable<T> =
  | { status: "loading" }
  | { status: "ready"; data: T }
  | { status: "error"; error: AppError };

// ---------------------------------------------------------------------------------------------
// Luduvo

/** Which game a part of Vodka belongs to. */
export type GameId = "roblox" | "luduvo";

export interface LuduvoStatus {
  installed: boolean;
  /** Luduvo's own build number. */
  version: string | null;
  /** False where Vodka can't start Luduvo yet (Windows). */
  supported: boolean;
  installCommand: string;
}

export interface LuduvoUser {
  id: number;
  name: string;
  displayName: string;
  balance: number | null;
}

export interface LuduvoPlace {
  id: number;
  title: string;
  creator: string;
  description: string;
  iconUrl: string | null;
  bannerUrl: string | null;
  playing: number;
  maxPlayers: number;
  visits: number;
  likes: number;
  dislikes: number;
  /** Your total time in this game, in seconds; only in recently played. */
  playtime: number | null;
}

export interface LuduvoShelf {
  id: string;
  title: string;
  subtitle: string | null;
  places: LuduvoPlace[];
}

/** The Luduvo game Vodka started, while it runs. */
export interface LuduvoRunning {
  placeId: number;
  title: string;
  pid: number;
  /** Unix seconds. */
  startedAt: number;
  /** checking, updating, content (downloading game content), starting, or playing. */
  stage: string;
  /** Content downloaded so far out of total, while stage is "content". */
  done: number | null;
  total: number | null;
}

export interface LuduvoFriend {
  id: number;
  name: string;
  displayName: string;
  avatarUrl: string | null;
  /** `online`, `in_game` or `offline`. */
  state: string;
  placeId: number | null;
  placeTitle: string | null;
  /** Luduvo lets you follow them into their server. */
  joinable: boolean;
}

/** What a Roblox game's page shows. */
export interface GameDetails {
  universeId: number;
  name: string;
  /** The place Play starts in. */
  rootPlaceId: number;
  creatorId: number | null;
  /** `Group` or `User`. */
  creatorType: string | null;
  iconUrl: string | null;
  /** All wide pictures, banner first. */
  gallery: string[];
  description: string;
  creator: string;
  creatorVerified: boolean;
  playing: number | null;
  visits: number | null;
  favorites: number | null;
  maxPlayers: number | null;
  genre: string | null;
  /** ISO date of the last update. */
  updated: string | null;
  bannerUrl: string | null;
}

export interface LuduvoDetails {
  place: LuduvoPlace;
  createdAt: number | null;
  updatedAt: number | null;
  ageRating: string | null;
  contentWarnings: string[];
  groupId: number | null;
  groupName: string | null;
}

export interface LuduvoRating {
  likes: number;
  dislikes: number;
  /** 1 liked, -1 disliked, 0 not rated. */
  vote: number;
}

export interface LuduvoServer {
  id: string;
  playing: number;
  maxPlayers: number;
  players: { id: number; name: string; avatarUrl: string | null }[];
}

/** A Roblox private server you own. */
export interface OwnedServer {
  id: number;
  name: string;
  active: boolean;
  /** ISO date the subscription runs out. */
  expires: string | null;
  willRenew: boolean;
  price: number | null;
  /** Whether your friends can join without an invite; null when Roblox didn't say. */
  friendsAllowed: boolean | null;
}

/** Whether you can make a Roblox private server for a game, and its monthly price (0 is free). */
export interface ServerOffer {
  canCreate: boolean;
  price: number | null;
}

/** A Roblox group, and where you stand in it. */
export interface GroupInfo {
  id: number;
  name: string;
  members: number;
  verified: boolean;
  /** Anyone can join straight away; otherwise joining sends a request the group approves. */
  publicEntry: boolean;
  /** Your rank's name when you're in it. */
  myRole: string | null;
}

/** Someone's Roblox profile. */
export interface Profile {
  id: number;
  name: string;
  displayName: string;
  description: string;
  /** ISO date the account was made. */
  created: string | null;
  verified: boolean;
  banned: boolean;
  avatarUrl: string | null;
  friends: number | null;
  followers: number | null;
  following: number | null;
  /** What they're doing now; only when signed in. */
  presence: Presence | null;
}

/** A group someone is in, with their rank. */
export interface UserGroup {
  id: number;
  name: string;
  members: number;
  verified: boolean;
  role: string;
  iconUrl: string | null;
}

/** One thing in Luduvo's store. */
export interface LuduvoItem {
  id: number;
  name: string;
  description: string;
  creator: string;
  /** C$; null when it isn't for sale. */
  price: number | null;
  limited: boolean;
  assetType: string;
  slot: string;
  pictureUrl: string | null;
  sales: number;
  likes: number;
  dislikes: number;
  owners: number | null;
  /** 1 liked, -1 disliked, 0 not rated. */
  myVote: number;
  /** Unix seconds the sale ends, for timed items. */
  expiresAt: number | null;
}

export interface LuduvoCategory {
  assetType: string;
  slot: string;
  name: string;
}

/** Something you own or wear on Luduvo. */
export interface LuduvoOwned {
  id: number;
  variant: number;
  name: string;
  assetType: string;
  slot: string;
  pictureUrl: string | null;
}

export interface LuduvoAvatar {
  pictureUrl: string | null;
  wearing: LuduvoOwned[];
}

export interface LuduvoOutfit {
  id: number;
  name: string;
  pictureUrl: string | null;
}

/** Someone's Luduvo profile. */
export interface LuduvoProfile {
  id: number;
  name: string;
  displayName: string;
  bio: string;
  status: string;
  tier: string;
  memberSince: number | null;
  lastActive: number | null;
  friends: number | null;
  places: number | null;
  items: number | null;
  friendsHidden: boolean;
  badges: string[];
  wearing: LuduvoOwned[];
  headshotUrl: string | null;
  pictureUrl: string | null;
  state: string;
  placeId: number | null;
  placeTitle: string | null;
  joinable: boolean;
}

export interface LuduvoGroup {
  id: number;
  name: string;
  members: number;
  role: string;
  iconUrl: string | null;
}
