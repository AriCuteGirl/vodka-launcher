// The only place the UI talks to the Rust backend.
import type { GroupInfo, ServerOffer, OwnedServer, GameDetails, LuduvoDetails, LuduvoRating, LuduvoServer, LuduvoFriend, LuduvoRunning, LuduvoPlace, LuduvoShelf, LuduvoStatus, LuduvoUser, Account, AppError, AvatarView, Chart, GameStanding, Item, ItemPage, Offer, PartySync, ChatContact, ChatMe, ChatMessage, ChatSync, FriendLocation, Person, ServerLists, Target, ExitEvent, Game, Instance, LaunchOptions, MocktailStatus, Settings, User } from "./types";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  if (import.meta.env.DEV) {
    const { demoInvoke } = await import("./demo");
    return demoInvoke(cmd, args) as Promise<T>;
  }
  throw { kind: "other", message: "Vodka Launcher has to run as a desktop app." };
}

async function on<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, (e) => handler(e.payload));
  }
  if (import.meta.env.DEV) {
    const { demoListen } = await import("./demo");
    return demoListen(event, (e) => handler(e.payload as T));
  }
  return () => {};
}

export const api = {
  mocktailStatus: () => call<MocktailStatus>("mocktail_status"),
  beginLogin: () => call<void>("begin_login"),
  currentAccount: () => call<User | null>("current_account"),
  listAccounts: () => call<Account[]>("list_accounts"),
  switchAccount: (accountId: number) => call<void>("switch_account", { accountId }),
  removeAccount: (accountId: number) => call<void>("remove_account", { accountId }),
  continuePlaying: () => call<Game[]>("continue_playing"),
  favorites: () => call<Game[]>("favorites"),
  searchGames: (query: string) => call<Game[]>("search_games", { query }),
  gameCharts: () => call<Chart[]>("game_charts"),
  launchGame: (game: Game, accountId: number | null = null, target: Target | null = null) => call<number>("launch_game", { game, accountId, target }),
  regroup: (universeId: number) => call<{ moved: number; playing: number; maxPlayers: number }>("regroup", { universeId }),
  listServers: (placeId: number) => call<ServerLists>("list_servers", { placeId }),
  friendLocation: (userId: number) => call<FriendLocation>("friend_location", { userId }),
  listFriends: () => call<Person[]>("list_friends"),
  listFriendRequests: () => call<Person[]>("list_friend_requests"),
  answerFriendRequest: (userId: number, accept: boolean) => call<void>("answer_friend_request", { userId, accept }),
  joinFriend: (userId: number) => call<number>("join_friend", { userId }),
  openPrivateServerPage: (placeId: number, name: string) => call<void>("open_private_server_page", { placeId, name }),
  onRobloxPageClosed: (h: () => void) => on<null>("roblox-page://closed", h),
  robloxGroup: (groupId: number) => call<GroupInfo>("roblox_group", { groupId }),
  /** Joins, letting Roblox's own page pass its human check in a hidden window when it asks for one. */
  joinRobloxGroup: (groupId: number, name: string) => call<GroupInfo>("join_roblox_group", { groupId, name }),
  openGroupPage: (groupId: number, name: string) => call<void>("open_group_page", { groupId, name }),
  privateServerOffer: (universeId: number) => call<ServerOffer>("private_server_offer", { universeId }),
  /** Resolves to a note when the server was made but friends couldn't be let in. */
  createPrivateServer: (universeId: number, name: string, agreedPrice: number, friendsAllowed: boolean) =>
    call<string | null>("create_private_server", { universeId, name, agreedPrice, friendsAllowed }),
  setServerFriends: (serverId: number, allowed: boolean) => call<void>("set_server_friends", { serverId, allowed }),
  ownedServers: (universeId: number) => call<OwnedServer[]>("owned_servers", { universeId }),
  setServerActive: (serverId: number, active: boolean) => call<void>("set_server_active", { serverId, active }),
  serverInviteLink: (serverId: number) => call<string>("server_invite_link", { serverId }),
  gameDetails: (universeId: number) => call<GameDetails>("game_details", { universeId }),
  gameStanding: (universeId: number) => call<GameStanding>("game_standing", { universeId }),
  setFavorite: (universeId: number, favorited: boolean) => call<GameStanding>("set_favorite", { universeId, favorited }),
  setVote: (universeId: number, like: boolean) => call<GameStanding>("set_vote", { universeId, like }),
  refreshInstanceGames: () => call<boolean>("refresh_instance_games"),
  avatar: () => call<AvatarView>("avatar"),
  avatarPicture: () => call<string | null>("avatar_picture"),
  wearOutfit: (outfitId: number) => call<string[]>("wear_outfit", { outfitId }),
  avatarWearing: () => call<Item[]>("avatar_wearing"),
  avatarInventory: (tab: string, cursor: string | null = null) => call<ItemPage>("avatar_inventory", { tab, cursor }),
  avatarSetItems: (assetIds: number[]) => call<number[]>("avatar_set_items", { assetIds }),
  avatarSetAccessories: (assetIds: number[]) => call<number[]>("avatar_set_accessories", { assetIds }),
  avatarSaveOutfit: (name: string) => call<void>("avatar_save_outfit", { name }),
  catalogSearch: (keyword: string, category: number, sort: number, cursor: string | null = null) =>
    call<ItemPage>("catalog_search", { keyword, category, sort, cursor }),
  robuxBalance: () => call<number>("robux_balance"),
  itemOffer: (assetId: number) => call<Offer>("item_offer", { assetId }),
  buyItem: (assetId: number, agreedPrice: number) => call<Offer>("buy_item", { assetId, agreedPrice }),
  headshots: (userIds: number[]) => call<Record<string, string>>("headshots", { userIds }),
  listInstances: () => call<Instance[]>("list_instances"),
  stopInstance: (id: number, restart: boolean) => call<void>("stop_instance", { id, restart }),
  getSettings: () => call<Settings>("get_settings"),
  setFlags: (flags: { onboarded?: boolean; multiInstanceAccepted?: boolean }) => call<Settings>("set_flags", { flags }),
  checkFflag: (name: string, value: string) => call<[string, string]>("check_fflag", { name, value }),
  saveLaunchOptions: (accountId: number | null, options: LaunchOptions | null) => call<Settings>("save_launch_options", { accountId, options }),
  rollbackClient: (accountId: number) => call<Settings>("rollback_client", { accountId }),
  chatEnabled: () => call<boolean>("chat_enabled"),
  chatStartVerify: () => call<{ challenge: string }>("chat_start_verify"),
  chatFinishVerify: () => call<{ me: ChatMe }>("chat_finish_verify"),
  chatSync: (since: number) => call<ChatSync>("chat_sync", { since }),
  chatAddContact: (code: string) => call<{ contact: ChatContact }>("chat_add_contact", { code }),
  chatRemoveContact: (userId: number) => call<unknown>("chat_remove_contact", { userId }),
  chatBlock: (userId: number, blocked: boolean) => call<unknown>("chat_block", { userId, blocked }),
  chatReport: (userId: number, reason: string) => call<unknown>("chat_report", { userId, reason }),
  chatSend: (to: number, body: string) => call<{ message: ChatMessage }>("chat_send", { to, body }),
  chatInvite: (to: number) => call<{ message: ChatMessage }>("chat_invite", { to }),
  partySync: (since: number) => call<PartySync>("party", { action: "sync", args: { since } }),
  party: (action: "create" | "invite" | "accept" | "decline" | "leave" | "kick" | "send" | "launch", args: Record<string, unknown> = {}) =>
    call<unknown>("party", { action, args }),
  chatSignOut: () => call<void>("chat_sign_out"),
  chatDeleteAccount: () => call<void>("chat_delete_account"),
  luduvoStatus: () => call<LuduvoStatus>("luduvo_status"),
  luduvoAccount: () => call<LuduvoUser | null>("luduvo_account"),
  luduvoSignIn: () => call<LuduvoUser>("luduvo_sign_in"),
  luduvoCancelSignIn: () => call<void>("luduvo_cancel_sign_in"),
  luduvoSignOut: () => call<void>("luduvo_sign_out"),
  luduvoDiscover: () => call<LuduvoShelf[]>("luduvo_discover"),
  luduvoSearch: (query: string) => call<LuduvoPlace[]>("luduvo_search", { query }),
  luduvoRecent: () => call<LuduvoPlace[]>("luduvo_recent"),
  luduvoPlay: (placeId: number, title: string) => call<LuduvoRunning>("luduvo_play", { placeId, title }),
  luduvoJoinFriend: (userId: number, placeId: number | null, title: string) => call<LuduvoRunning>("luduvo_join_friend", { userId, placeId, title }),
  luduvoDetails: (placeId: number) => call<LuduvoDetails>("luduvo_details", { placeId }),
  luduvoRating: (placeId: number) => call<LuduvoRating>("luduvo_rating", { placeId }),
  luduvoRate: (placeId: number, value: number) => call<LuduvoRating>("luduvo_rate", { placeId, value }),
  luduvoPlayPrivate: (code: string, placeId: number, title: string) => call<LuduvoRunning>("luduvo_play_private", { code, placeId, title }),
  luduvoJoinGroup: (groupId: number) => call<void>("luduvo_join_group", { groupId }),
  openExternal: (url: string) => call<void>("open_external", { url }),
  luduvoServers: (placeId: number) => call<LuduvoServer[]>("luduvo_servers", { placeId }),
  luduvoPlayServer: (serverId: string, placeId: number, title: string) => call<LuduvoRunning>("luduvo_play_server", { serverId, placeId, title }),
  luduvoFriends: () => call<LuduvoFriend[]>("luduvo_friends"),
  luduvoFriendRequests: () => call<LuduvoFriend[]>("luduvo_friend_requests"),
  luduvoAnswerRequest: (userId: number, accept: boolean) => call<void>("luduvo_answer_request", { userId, accept }),
  luduvoBalance: () => call<number | null>("luduvo_balance"),
  luduvoClaimDaily: () => call<Record<string, unknown> | null>("luduvo_claim_daily"),
  luduvoRunning: () => call<LuduvoRunning | null>("luduvo_running"),
  luduvoStop: () => call<void>("luduvo_stop"),
  onLuduvoChanged: (h: () => void) => on<null>("luduvo://changed", h),
  luduvoOpenStudio: () => call<void>("luduvo_open_studio"),
  onInstancesChanged: (h: () => void) => on<null>("instances://changed", h),
  onInstanceExit: (h: (event: ExitEvent) => void) => on<ExitEvent>("instances://exit", h),
  onInstanceError: (h: (error: AppError) => void) => on<AppError>("instances://error", h),
  onSignedIn: (h: (user: User) => void) => on<User>("auth://signed-in", h),
  onLoginClosed: (h: () => void) => on<null>("auth://closed", h),
  onAuthError: (h: (error: AppError) => void) => on<AppError>("auth://error", h),
};
