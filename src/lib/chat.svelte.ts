// Vodka's own chat: state and actions. Opt-in per account; nothing is contacted until it's turned on.
import { api } from "./api";
import { app, launchMany, toast } from "./state.svelte";
import { toAppError, type ChatContact, type ChatInvite, type ChatMe, type ChatMessage } from "./types";

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
  /** Whose conversation is open on the Chat page. */
  open: null as number | null,
  /** True while the Chat page is on screen (faster refresh, no toasts). */
  visible: false,
  unreadTotal: 0,
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
  chat.unreadTotal = chat.contacts.reduce((n, c) => n + unreadFor(c.robloxId), 0);
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
    if (chat.visible && chat.open != null) markSeen(chat.open);
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

/** Starts the background refresh. Call once. */
export function startChat() {
  let ticks = 0;
  setInterval(() => {
    ticks++;
    // Every 4 s while the Chat page is open, every 20 s otherwise, and not at all while chat is off.
    const due = chat.visible ? true : ticks % 5 === 0;
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
