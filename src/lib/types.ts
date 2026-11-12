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
}

/** `null` means "don't override": the account falls back to the defaults, the defaults to Mocktail's own config. */
export interface LaunchOptions {
  fps: string | null;
  backend: string | null;
  vsync: string | null;
  presentMode: string | null;
  multithreadedRendering: boolean | null;
  gamemode: string | null;
  memoryLimitMb: number | null;
  holdUpdates: boolean | null;
  fflags: Record<string, string>;
}

export const NO_OPTIONS: LaunchOptions = {
  fps: null,
  backend: null,
  vsync: null,
  presentMode: null,
  multithreadedRendering: null,
  gamemode: null,
  memoryLimitMb: null,
  holdUpdates: null,
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
