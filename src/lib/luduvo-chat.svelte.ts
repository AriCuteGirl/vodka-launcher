// Luduvo Messages: the chat list and unread count, kept fresh while Vodka is open.
import { api } from "./api";
import type { LuduvoConversation } from "./types";

export const luduvoChat = $state({
  list: [] as LuduvoConversation[],
  loaded: false,
  error: null as string | null,
  /** The chat open on the Messages page, so it doesn't count as unread. */
  open: null as number | null,
});

export const unreadChats = () => luduvoChat.list.reduce((n, c) => n + (c.id === luduvoChat.open ? 0 : c.unread), 0);

let timer: ReturnType<typeof setInterval> | null = null;

export async function loadChats() {
  try {
    const list = await api.luduvoConversations();
    luduvoChat.list = list.map((c) => (c.id === luduvoChat.open ? { ...c, unread: 0 } : c));
    luduvoChat.loaded = true;
    luduvoChat.error = null;
  } catch (e) {
    luduvoChat.error = (e as { message?: string })?.message ?? "Couldn't load your messages.";
  }
}

/** Starts looking for new messages every 15 seconds; once is enough. */
export function watchChats() {
  void loadChats();
  if (timer) return;
  timer = setInterval(() => {
    if (document.visibilityState === "visible") void loadChats();
  }, 15_000);
}

export function stopChats() {
  if (timer) clearInterval(timer);
  timer = null;
  luduvoChat.list = [];
  luduvoChat.loaded = false;
}

/** A Luduvo user's small round picture (Luduvo redirects to the current one). */
export const headshot = (userId: number | null) => (userId ? `https://api.luduvo.com/users/${userId}/avatar/headshot?size=96` : null);
