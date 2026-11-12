// Vodka Launcher chat server (Supabase Edge Function).
//
// One endpoint, JSON in and out: POST { "action": "...", ... }.
// Signed-in actions carry the session token in the `x-vodka-token` header.
// The database is only reachable from here (service role); clients never touch it directly.
// A Roblox session cookie is never sent to, or accepted by, this server.
import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";

const db = createClient(Deno.env.get("SUPABASE_URL")!, Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!, {
  auth: { persistSession: false },
});

const WORDS = [
  "amber", "anchor", "apple", "arrow", "autumn", "basket", "beacon", "birch", "breeze", "bridge", "candle", "canyon",
  "cedar", "cloud", "clover", "copper", "coral", "cotton", "crystal", "daisy", "delta", "ember", "falcon", "feather",
  "forest", "garden", "glacier", "harbor", "hazel", "island", "ivory", "jasmine", "lantern", "lemon", "lotus", "maple",
  "marble", "meadow", "mirror", "morning", "nectar", "ocean", "olive", "orchid", "pebble", "pepper", "pine", "planet",
  "pocket", "quartz", "rabbit", "ribbon", "river", "saffron", "silver", "sparrow", "summit", "sunset", "thunder",
  "timber", "tulip", "valley", "velvet", "willow",
];
const ONLINE_SECONDS = 90;
const MESSAGE_DAYS = 14;
const SENDS_PER_MINUTE = 20;

class Fail extends Error {
  constructor(public status: number, public code: string, message: string) {
    super(message);
  }
}
const fail = (status: number, code: string, message: string): never => {
  throw new Fail(status, code, message);
};

function json(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });
}

function randomInt(max: number) {
  const buf = new Uint32Array(1);
  crypto.getRandomValues(buf);
  return buf[0] % max;
}
const randomHex = (bytes: number) =>
  [...crypto.getRandomValues(new Uint8Array(bytes))].map((b) => b.toString(16).padStart(2, "0")).join("");

async function sha256(text: string) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

function newFriendCode() {
  // No 0/O/1/I, so codes survive being read out loud.
  const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  const part = () => Array.from({ length: 4 }, () => alphabet[randomInt(alphabet.length)]).join("");
  return `VDK-${part()}-${part()}`;
}

function robloxId(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) fail(400, "bad_request", "That isn't a Roblox user id.");
  return value as number;
}

function must<T>(result: { data: T; error: { message: string } | null }): T {
  if (result.error) {
    console.error("db", result.error.message);
    fail(500, "server", "The chat server had a database problem. Try again.");
  }
  return result.data;
}

async function robloxProfile(id: number) {
  const res = await fetch(`https://users.roblox.com/v1/users/${id}`, { headers: { Accept: "application/json" } });
  if (res.status === 404) fail(404, "no_such_user", "Roblox has no user with that id.");
  if (!res.ok) fail(502, "roblox_unavailable", "Roblox didn't answer. Try again in a moment.");
  const p = await res.json();
  return { name: String(p.name ?? ""), displayName: String(p.displayName ?? p.name ?? ""), description: String(p.description ?? ""), banned: !!p.isBanned };
}

async function startVerify(body: Record<string, unknown>) {
  const id = robloxId(body.robloxId);
  const existing = must(await db.from("vodka_verifications").select("challenge, expires_at").eq("roblox_id", id).maybeSingle());
  // Phrases are plain everyday words only: Roblox's text filter hides brand and alcohol words in profiles.
  // (An early version prefixed "vodka"; such a pending phrase is replaced rather than reused.)
  if (existing && new Date(existing.expires_at) > new Date() && !existing.challenge.startsWith("vodka ")) {
    return { challenge: existing.challenge };
  }
  const challenge = Array.from({ length: 6 }, () => WORDS[randomInt(WORDS.length)]).join(" ");
  must(
    await db.from("vodka_verifications").upsert({
      roblox_id: id,
      challenge,
      attempts: 0,
      expires_at: new Date(Date.now() + 15 * 60_000).toISOString(),
    }),
  );
  return { challenge };
}

async function finishVerify(body: Record<string, unknown>) {
  const id = robloxId(body.robloxId);
  const pending = must(await db.from("vodka_verifications").select("challenge, attempts, expires_at").eq("roblox_id", id).maybeSingle());
  if (!pending || new Date(pending.expires_at) < new Date()) fail(410, "expired", "That code has expired. Start again to get a new one.");
  if (pending!.attempts >= 20) fail(429, "too_many", "Too many tries. Wait for the code to expire, then start again.");
  must(await db.from("vodka_verifications").update({ attempts: pending!.attempts + 1 }).eq("roblox_id", id));

  const profile = await robloxProfile(id);
  if (profile.banned) fail(403, "banned", "That Roblox account is banned.");
  const squash = (s: string) => s.toLowerCase().replace(/\s+/g, " ");
  if (!squash(profile.description).includes(squash(pending!.challenge))) {
    fail(409, "not_found", "The code isn't in your Roblox About text yet. Roblox can take a minute to show changes; save it and try again.");
  }

  const existing = must(await db.from("vodka_users").select("code").eq("roblox_id", id).maybeSingle());
  let code = existing?.code as string | undefined;
  if (code) {
    must(await db.from("vodka_users").update({ name: profile.name, display_name: profile.displayName, last_seen: new Date().toISOString() }).eq("roblox_id", id));
  } else {
    for (let tries = 0; ; tries++) {
      code = newFriendCode();
      const insert = await db.from("vodka_users").insert({ roblox_id: id, name: profile.name, display_name: profile.displayName, code });
      if (!insert.error) break;
      if (tries >= 4) must(insert);
    }
  }
  const token = randomHex(32);
  must(await db.from("vodka_sessions").insert({ token_hash: await sha256(token), roblox_id: id }));
  must(await db.from("vodka_verifications").delete().eq("roblox_id", id));
  return { token, me: { robloxId: id, name: profile.name, displayName: profile.displayName, code } };
}

async function signedIn(req: Request): Promise<number> {
  const token = req.headers.get("x-vodka-token") ?? "";
  if (!/^[0-9a-f]{64}$/.test(token)) fail(401, "signed_out", "You're not signed in to Vodka chat.");
  const hash = await sha256(token);
  const session = must(await db.from("vodka_sessions").select("roblox_id").eq("token_hash", hash).maybeSingle());
  if (!session) fail(401, "signed_out", "Your Vodka chat sign-in is no longer valid. Verify again.");
  return session!.roblox_id as number;
}

async function isBlockedEitherWay(a: number, b: number) {
  const rows = must(
    await db.from("vodka_blocks").select("owner").or(`and(owner.eq.${a},blocked.eq.${b}),and(owner.eq.${b},blocked.eq.${a})`).limit(1),
  );
  return rows!.length > 0;
}

async function sync(me: number, body: Record<string, unknown>) {
  const since = typeof body.since === "number" && Number.isSafeInteger(body.since) && body.since >= 0 ? body.since : 0;
  const now = new Date();
  must(await db.from("vodka_users").update({ last_seen: now.toISOString() }).eq("roblox_id", me));
  must(await db.from("vodka_sessions").update({ last_used: now.toISOString() }).eq("roblox_id", me));
  if (randomInt(50) === 0) {
    await db.from("vodka_messages").delete().lt("created_at", new Date(Date.now() - MESSAGE_DAYS * 86_400_000).toISOString());
  }
  const self = must(await db.from("vodka_users").select("roblox_id, name, display_name, code").eq("roblox_id", me).single());
  const links = must(await db.from("vodka_contacts").select("other").eq("owner", me));
  const ids = links!.map((l) => l.other as number);
  const people = ids.length
    ? must(await db.from("vodka_users").select("roblox_id, name, display_name, last_seen").in("roblox_id", ids))!
    : [];
  const blocked = must(await db.from("vodka_blocks").select("blocked").eq("owner", me))!.map((b) => b.blocked as number);
  const messages = must(
    await db
      .from("vodka_messages")
      .select("id, sender, recipient, kind, body, payload, created_at")
      .gt("id", since)
      .or(`sender.eq.${me},recipient.eq.${me}`)
      .order("id", { ascending: true })
      .limit(300),
  );
  return {
    me: { robloxId: self!.roblox_id, name: self!.name, displayName: self!.display_name, code: self!.code },
    contacts: people.map((p) => ({
      robloxId: p.roblox_id,
      name: p.name,
      displayName: p.display_name,
      online: now.getTime() - new Date(p.last_seen).getTime() < ONLINE_SECONDS * 1000,
    })),
    blocked,
    // Messages from people blocked after they wrote are hidden rather than deleted.
    messages: messages!
      .filter((m) => !blocked.includes(m.sender as number))
      .map((m) => ({ id: m.id, sender: m.sender, recipient: m.recipient, kind: m.kind, body: m.body, payload: m.payload, createdAt: m.created_at })),
  };
}

async function addContact(me: number, body: Record<string, unknown>) {
  const code = String(body.code ?? "").trim().toUpperCase();
  if (!/^VDK-[A-Z2-9]{4}-[A-Z2-9]{4}$/.test(code)) fail(400, "bad_code", "Vodka codes look like VDK-ABCD-2345.");
  const other = must(await db.from("vodka_users").select("roblox_id, name, display_name").eq("code", code).maybeSingle());
  if (!other) fail(404, "no_such_code", "Nobody has that Vodka code. Check it with the person who gave it to you.");
  const id = other!.roblox_id as number;
  if (id === me) fail(400, "self", "That's your own code.");
  if (await isBlockedEitherWay(me, id)) fail(403, "blocked", "You can't add this person.");
  must(await db.from("vodka_contacts").upsert([{ owner: me, other: id }, { owner: id, other: me }]));
  return { contact: { robloxId: id, name: other!.name, displayName: other!.display_name, online: false } };
}

async function removeContact(me: number, body: Record<string, unknown>) {
  const id = robloxId(body.robloxId);
  must(await db.from("vodka_contacts").delete().or(`and(owner.eq.${me},other.eq.${id}),and(owner.eq.${id},other.eq.${me})`));
  return { ok: true };
}

async function block(me: number, body: Record<string, unknown>, on: boolean) {
  const id = robloxId(body.robloxId);
  if (id === me) fail(400, "self", "You can't block yourself.");
  if (on) {
    const exists = must(await db.from("vodka_users").select("roblox_id").eq("roblox_id", id).maybeSingle());
    if (!exists) fail(404, "no_such_user", "That person doesn't use Vodka chat.");
    must(await db.from("vodka_blocks").upsert({ owner: me, blocked: id }));
    must(await db.from("vodka_contacts").delete().or(`and(owner.eq.${me},other.eq.${id}),and(owner.eq.${id},other.eq.${me})`));
  } else {
    must(await db.from("vodka_blocks").delete().eq("owner", me).eq("blocked", id));
  }
  return { ok: true };
}

function invitePayload(raw: unknown) {
  const bad = () => fail(400, "bad_invite", "That game invite is malformed.");
  if (!raw || typeof raw !== "object") bad();
  const p = raw as Record<string, unknown>;
  const game = p.game as Record<string, unknown> | undefined;
  const int = (v: unknown) => typeof v === "number" && Number.isSafeInteger(v) && v > 0;
  if (!game || !int(game.universeId) || !int(game.placeId) || typeof game.name !== "string" || game.name.length > 200) bad();
  const icon = game!.iconUrl;
  if (icon != null && !(typeof icon === "string" && /^https:\/\/[a-z0-9-]+\.rbxcdn\.com\/[\w\-./]+$/i.test(icon) && icon.length < 400)) bad();
  if (!int(p.placeId)) bad();
  if (p.gameId != null && !(typeof p.gameId === "string" && /^[A-Za-z0-9-]{1,64}$/.test(p.gameId))) bad();
  return {
    game: { universeId: game!.universeId, placeId: game!.placeId, name: game!.name, playing: null, iconUrl: icon ?? null },
    placeId: p.placeId,
    gameId: p.gameId ?? null,
  };
}

async function send(me: number, body: Record<string, unknown>) {
  const to = robloxId(body.to);
  const kind = body.kind === "invite" ? "invite" : "text";
  const text = String(body.body ?? "").replace(/[\u0000-\u0008\u000b-\u001f\u007f]/g, "").trim();
  if (kind === "text" && text.length === 0) fail(400, "empty", "Write something first.");
  if (text.length > 1000) fail(400, "too_long", "Messages can be up to 1000 characters.");
  const payload = kind === "invite" ? invitePayload(body.payload) : null;

  const link = must(await db.from("vodka_contacts").select("owner").eq("owner", me).eq("other", to).maybeSingle());
  if (!link) fail(403, "not_contact", "You can only message people you've added with a Vodka code.");
  if (await isBlockedEitherWay(me, to)) fail(403, "blocked", "You can't message this person.");
  const counted = await db.from("vodka_messages").select("id", { count: "exact", head: true }).eq("sender", me).gt("created_at", new Date(Date.now() - 60_000).toISOString());
  if ((counted.count ?? 0) >= SENDS_PER_MINUTE) fail(429, "slow_down", "You're sending messages too fast. Wait a moment.");

  const row = must(
    await db.from("vodka_messages").insert({ sender: me, recipient: to, kind, body: text, payload }).select("id, sender, recipient, kind, body, payload, created_at").single(),
  );
  return { message: { id: row!.id, sender: row!.sender, recipient: row!.recipient, kind: row!.kind, body: row!.body, payload: row!.payload, createdAt: row!.created_at } };
}

async function report(me: number, body: Record<string, unknown>) {
  const id = robloxId(body.robloxId);
  const reason = String(body.reason ?? "").trim().slice(0, 500);
  if (!reason) fail(400, "empty", "Say what happened.");
  const today = await db.from("vodka_reports").select("id", { count: "exact", head: true }).eq("reporter", me).gt("created_at", new Date(Date.now() - 86_400_000).toISOString());
  if ((today.count ?? 0) >= 20) fail(429, "slow_down", "That's a lot of reports for one day. Try again tomorrow.");
  must(await db.from("vodka_reports").insert({ reporter: me, reported: id, reason }));
  return { ok: true };
}

Deno.serve(async (req: Request) => {
  try {
    if (req.method !== "POST") fail(405, "bad_request", "Use POST.");
    const text = await req.text();
    if (text.length > 8000) fail(413, "bad_request", "That request is too large.");
    let body: Record<string, unknown>;
    try {
      body = JSON.parse(text);
    } catch {
      body = fail(400, "bad_request", "That request isn't valid JSON.");
    }
    if (!body || typeof body !== "object" || Array.isArray(body)) fail(400, "bad_request", "That request isn't an object.");

    switch (body.action) {
      case "start_verify":
        return json(await startVerify(body));
      case "finish_verify":
        return json(await finishVerify(body));
    }
    const me = await signedIn(req);
    switch (body.action) {
      case "sync":
        return json(await sync(me, body));
      case "add_contact":
        return json(await addContact(me, body));
      case "remove_contact":
        return json(await removeContact(me, body));
      case "block":
        return json(await block(me, body, true));
      case "unblock":
        return json(await block(me, body, false));
      case "send":
        return json(await send(me, body));
      case "report":
        return json(await report(me, body));
      case "sign_out":
        must(await db.from("vodka_sessions").delete().eq("token_hash", await sha256(req.headers.get("x-vodka-token")!)));
        return json({ ok: true });
      case "delete_account":
        must(await db.from("vodka_users").delete().eq("roblox_id", me));
        return json({ ok: true });
    }
    return fail(400, "bad_request", "Unknown action.");
  } catch (e) {
    if (e instanceof Fail) return json({ error: { code: e.code, message: e.message } }, e.status);
    console.error("unhandled", e);
    return json({ error: { code: "server", message: "The chat server hit an unexpected problem." } }, 500);
  }
});
