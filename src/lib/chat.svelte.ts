// Vodka's own chat: state and actions. Opt-in per account; nothing is contacted until it's turned on.
import { api } from "./api";
import { app, launchMany, toast } from "./state.svelte";
import { toAppError, type ChatContact, type ChatInvite, type ChatMe, type ChatMessage, type Party, type PartyEvent, type PartyInvite } from "./types";

export const chat = $state({
  /** "unknown" until checked, "off" when not turned on for the active account. */
  status: "unknown" as "unknown" | "off" | "on" | "error",
  error: null as string | null,
  /** The account this state belongs to; a switch resets everything. */
  accountId: null as number | null,
  me: null as ChatMe | null,
  contacts: [] as ChatContact[],
  messages: [] as ChatMessage[],
  lastId: 0,
  /** Newest message id the user has seen, per contact. */
  seen: {} as Record<string, number>,
  /** What is open on the Chat page: a contact's id, the party, or nothing. */
  open: null as number | "party" | null,
  party: null as Party | null,
  partyEvents: [] as PartyEvent[],
  partyLastId: 0,
  partyInvites: [] as PartyInvite[],
  partyUnread: 0,
  /** Members: start the game the leader starts, without asking. Remembered on this computer. */
  follow: typeof localStorage === "undefined" || localStorage.getItem("vodka-party-follow") !== "off",
  /** True while the Chat page is on screen (faster refresh, no toasts). */
  visible: false,
  unreadTotal: 0,
  /** Roblox profile pictures by user id, looked up from Roblox (the chat server only knows names). */
  pictures: {} as Record<string, string>,
});

const seenKey = (accountId: number) => `vodka-chat-seen:${accountId}`;

function loadSeen(accountId: number): Record<string, number> {
  try {
    return JSON.parse(localStorage.getItem(seenKey(accountId)) ?? "{}");
  } catch {
    return {};
  }
}

/** The other person in a message. */
export const peerOf = (m: ChatMessage, me: number) => (m.sender === me ? m.recipient : m.sender);

export function unreadFor(contactId: number): number {
  const me = chat.me?.robloxId;
  if (me == null) return 0;
  const seen = chat.seen[String(contactId)] ?? 0;
  return chat.messages.filter((m) => m.sender === contactId && m.id > seen).length;
}

function recount() {
  chat.unreadTotal = chat.contacts.reduce((n, c) => n + unreadFor(c.robloxId), 0) + chat.partyUnread + chat.partyInvites.length;
}

export function markSeen(contactId: number) {
  const newest = chat.messages.findLast((m) => m.sender === contactId || m.recipient === contactId)?.id ?? 0;
  if ((chat.seen[String(contactId)] ?? 0) >= newest) return;
  chat.seen[String(contactId)] = newest;
  if (chat.accountId != null) {
    try {
      localStorage.setItem(seenKey(chat.accountId), JSON.stringify(chat.seen));
    } catch {
      // Unread counts just won't survive a restart.
    }
  }
  recount();
}

function reset(accountId: number | null) {
  chat.accountId = accountId;
  chat.status = "unknown";
  chat.error = null;
  chat.me = null;
  chat.contacts = [];
  chat.messages = [];
  chat.lastId = 0;
  chat.open = null;
  chat.party = null;
  chat.partyEvents = [];
  chat.partyLastId = 0;
  chat.partyInvites = [];
  chat.partyUnread = 0;
  chat.seen = accountId == null ? {} : loadSeen(accountId);
  chat.unreadTotal = 0;
}

let syncing = false;

/** Fetches contacts and new messages. Safe to call often; overlapping calls are skipped. */
export async function syncChat() {
  const accountId = app.account?.id ?? null;
  if (accountId !== chat.accountId) reset(accountId);
  if (accountId == null || syncing) return;
  syncing = true;
  try {
    if (chat.status === "unknown") chat.status = (await api.chatEnabled()) ? "on" : "off";
    if (chat.status !== "on" && chat.status !== "error") return;
    const first = chat.lastId === 0;
    const data = await api.chatSync(chat.lastId);
    if (app.account?.id !== accountId) return;
    chat.status = "on";
    chat.error = null;
    chat.me = data.me;
    chat.contacts = data.contacts;
    const missing = data.contacts.map((c) => c.robloxId).filter((id) => !(String(id) in chat.pictures));
    if (missing.length) {
      void api.headshots(missing).then((found) => (chat.pictures = { ...chat.pictures, ...found })).catch(() => {});
    }
    if (data.messages.length) {
      const known = new Set(chat.messages.map((m) => m.id));
      const fresh = data.messages.filter((m) => !known.has(m.id));
      chat.messages = [...chat.messages, ...fresh];
      chat.lastId = Math.max(chat.lastId, ...data.messages.map((m) => m.id));
      const incoming = fresh.filter((m) => m.sender !== data.me.robloxId);
      // Don't announce history on the first load, or messages in the conversation being read.
      if (!first) {
        for (const m of incoming) {
          if (chat.visible && chat.open === m.sender) continue;
          const who = data.contacts.find((c) => c.robloxId === m.sender)?.displayName ?? "Someone";
          toast(m.kind === "invite" ? `${who} invited you to ${m.payload?.game.name ?? "a game"}.` : `${who}: ${m.body.slice(0, 80)}`);
        }
      }
    }
    if (chat.visible && typeof chat.open === "number") markSeen(chat.open);
    await syncParty(accountId);
    await announceMyGame();
    recount();
  } catch (e) {
    const error = toAppError(e);
    if (error.kind === "chat_signed_out") {
      chat.status = "off";
      chat.me = null;
    } else if (chat.status === "on" && chat.me) {
      // Keep showing what we have; the next refresh may work.
      chat.error = error.message;
    } else {
      chat.status = "error";
      chat.error = error.message;
    }
  } finally {
    syncing = false;
  }
}

/** Party state: who is in it, what was said, and invites waiting for an answer. */
async function syncParty(accountId: number) {
  let data;
  try {
    data = await api.partySync(chat.partyLastId);
  } catch {
    // The party is an extra on top of chat; a failed refresh just tries again next round.
    return;
  }
  if (app.account?.id !== accountId) return;
  const me = chat.me?.robloxId;
  const known = new Set(chat.partyInvites.map((i) => i.partyId));
  for (const inv of data.invites) {
    if (!known.has(inv.partyId) && !chat.visible) toast(`${inv.from} invited you to a party. Open Chat to answer.`);
  }
  chat.partyInvites = data.invites;
  if (!data.party) {
    if (chat.party) toast("Your party ended.");
    chat.party = null;
    chat.partyEvents = [];
    chat.partyLastId = 0;
    chat.partyUnread = 0;
    if (chat.open === "party") chat.open = null;
    return;
  }
  const first = chat.party?.id !== data.party.id;
  if (first) {
    chat.partyEvents = [];
    chat.partyUnread = 0;
  }
  chat.party = data.party;
  if (data.events.length) {
    const have = new Set(chat.partyEvents.map((e) => e.id));
    const fresh = data.events.filter((e) => !have.has(e.id));
    chat.partyEvents = [...chat.partyEvents, ...fresh];
    chat.partyLastId = Math.max(chat.partyLastId, ...data.events.map((e) => e.id));
    const watching = chat.visible && chat.open === "party";
    for (const e of fresh) {
      if (first || e.sender === me || e.kind === "system") continue;
      if (watching && e.kind !== "launch") continue;
      const who = data.party.members.find((m) => m.robloxId === e.sender)?.displayName ?? "Someone";
      if (e.kind === "launch" && e.payload) {
        // Follow the leader straight in, unless this account is already playing something.
        const busy = app.instances.some((i) => i.accountId === me);
        if (chat.follow && e.sender === data.party.leader && !busy) {
          toast(`Following ${who} into ${e.payload.game.name}.`, "ok");
          acceptInvite(e.payload);
          continue;
        }
        chat.partyUnread++;
        toast(`${who} is taking the party into ${e.payload.game.name}. Open Chat to join.`);
      } else {
        chat.partyUnread++;
        toast(`Party · ${who}: ${e.body.slice(0, 80)}`);
      }
    }
  }
}

export function setFollow(on: boolean) {
  chat.follow = on;
  try {
    localStorage.setItem("vodka-party-follow", on ? "on" : "off");
  } catch {
    // It just won't be remembered.
  }
}

/** Games this window already told the party about, as "instance:universe". */
const announced = new Set<string>();

/** As party leader, tell the party whenever my game is up (or I switched to another one inside Roblox). */
async function announceMyGame() {
  const me = chat.me?.robloxId;
  if (!chat.party || me == null || chat.party.leader !== me || chat.party.members.length < 2) return;
  const mine = app.instances.find((i) => i.accountId === me && i.phase === "playing");
  // A few seconds after it appears, Roblox knows which server it is.
  if (!mine || Date.now() - mine.startedAtMs < 6000) return;
  const key = `${mine.id}:${mine.game.universeId}`;
  if (announced.has(key)) return;
  announced.add(key);
  try {
    await api.party("launch");
  } catch {
    // Let the next round try again.
    announced.delete(key);
  }
}

/** Marks the party conversation as read. */
export function seeParty() {
  if (chat.partyUnread) {
    chat.partyUnread = 0;
    recount();
  }
}

/** Runs a party action, then refreshes. Returns whether it worked. */
export async function partyDo(action: Parameters<typeof api.party>[0], args: Record<string, unknown> = {}, done?: string): Promise<boolean> {
  try {
    await api.party(action, args);
    await syncChat();
    if (done) toast(done, "ok");
    return true;
  } catch (e) {
    toast(toAppError(e).message, "danger");
    return false;
  }
}

/** Starts the background refresh. Call once. */
export function startChat() {
  let ticks = 0;
  setInterval(() => {
    ticks++;
    // Every 4 s while the Chat page is open, every 20 s otherwise, and not at all while chat is off.
    // Every round while the Chat page is open or a party is on (so following the leader is quick).
    const due = chat.visible || chat.party ? true : ticks % 5 === 0;
    if (due && app.account && app.online && (chat.status === "on" || chat.status === "unknown" || app.account.id !== chat.accountId)) void syncChat();
  }, 4000);
  void syncChat();
}

export async function sendText(to: number, body: string): Promise<boolean> {
  try {
    const { message } = await api.chatSend(to, body);
    chat.messages = [...chat.messages, message];
    chat.lastId = Math.max(chat.lastId, message.id);
    markSeen(to);
    return true;
  } catch (e) {
    toast(toAppError(e).message, "danger");
    return false;
  }
}

export async function sendInvite(to: number) {
  try {
    const { message } = await api.chatInvite(to);
    chat.messages = [...chat.messages, message];
    chat.lastId = Math.max(chat.lastId, message.id);
    toast("Invite sent.", "ok");
  } catch (e) {
    toast(toAppError(e).message, "danger");
  }
}

/** Joins the game in an invite as the active account: the sender's server if it was shared, else any server. */
export function acceptInvite(invite: ChatInvite) {
  if (!app.account) return;
  const target = invite.gameId ? ({ kind: "server", placeId: invite.placeId, gameId: invite.gameId } as const) : null;
  void launchMany(invite.game, [app.account.id], target);
}
