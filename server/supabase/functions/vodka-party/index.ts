// Vodka Launcher party server (Supabase Edge Function).
//
// A party is up to six Vodka users who chat together and launch a game together.
// Same rules as the chat function: JSON POST with an `action`, the chat sign-in in
// `x-vodka-token`, database reachable only from here, no Roblox session ever involved.
import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";

const db = createClient(Deno.env.get("SUPABASE_URL")!, Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!, {
  auth: { persistSession: false },
});

const MAX_MEMBERS = 6;
const ONLINE_SECONDS = 90;
const SENDS_PER_MINUTE = 30;

class Fail extends Error {
  constructor(public status: number, public code: string, message: string) {
    super(message);
  }
}
const fail = (status: number, code: string, message: string): never => {
  throw new Fail(status, code, message);
};
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });

async function sha256(text: string) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

function must<T>(result: { data: T; error: { message: string } | null }): T {
  if (result.error) {
    console.error("db", result.error.message);
    fail(500, "server", "The party server had a database problem. Try again.");
  }
  return result.data;
}

function userId(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) fail(400, "bad_request", "That isn't a user id.");
  return value as number;
}

async function signedIn(req: Request): Promise<number> {
  const token = req.headers.get("x-vodka-token") ?? "";
  if (!/^[0-9a-f]{64}$/.test(token)) fail(401, "signed_out", "You're not signed in to Vodka chat.");
  const session = must(await db.from("vodka_sessions").select("roblox_id").eq("token_hash", await sha256(token)).maybeSingle());
  if (!session) fail(401, "signed_out", "Your Vodka chat sign-in is no longer valid. Verify again.");
  return session!.roblox_id as number;
}

async function nameOf(id: number): Promise<string> {
  const u = must(await db.from("vodka_users").select("display_name").eq("roblox_id", id).maybeSingle());
  return (u?.display_name as string) ?? "Someone";
}

/** The party `me` is in, or null. */
async function myParty(me: number): Promise<{ id: number; leader: number } | null> {
  const row = must(await db.from("vodka_party_members").select("party_id").eq("roblox_id", me).maybeSingle());
  if (!row) return null;
  const party = must(await db.from("vodka_parties").select("id, leader").eq("id", row.party_id).maybeSingle());
  return party ? { id: party.id as number, leader: party.leader as number } : null;
}

async function needParty(me: number) {
  const party = await myParty(me);
  if (!party) fail(409, "no_party", "You're not in a party.");
  return party!;
}

const system = (partyId: number, body: string) =>
  db.from("vodka_party_events").insert({ party_id: partyId, sender: null, kind: "system", body });

async function create(me: number) {
  if (await myParty(me)) fail(409, "in_party", "Leave your current party first.");
  const party = must(await db.from("vodka_parties").insert({ leader: me }).select("id").single());
  must(await db.from("vodka_party_members").insert({ party_id: party!.id, roblox_id: me }));
  await system(party!.id as number, `${await nameOf(me)} started the party.`);
  return { ok: true };
}

async function invite(me: number, body: Record<string, unknown>) {
  const to = userId(body.robloxId);
  const party = await needParty(me);
  const contact = must(await db.from("vodka_contacts").select("owner").eq("owner", me).eq("other", to).maybeSingle());
  if (!contact) fail(403, "not_contact", "You can only invite people you've added with a Vodka code.");
  const blocked = must(
    await db.from("vodka_blocks").select("owner").or(`and(owner.eq.${me},blocked.eq.${to}),and(owner.eq.${to},blocked.eq.${me})`).limit(1),
  );
  if (blocked!.length) fail(403, "blocked", "You can't invite this person.");
  const already = must(await db.from("vodka_party_members").select("party_id").eq("roblox_id", to).maybeSingle());
  if (already?.party_id === party.id) fail(409, "already_member", "They're already in your party.");
  const members = await db.from("vodka_party_members").select("roblox_id", { count: "exact", head: true }).eq("party_id", party.id);
  const pending = await db.from("vodka_party_invites").select("roblox_id", { count: "exact", head: true }).eq("party_id", party.id);
  if ((members.count ?? 0) + (pending.count ?? 0) >= MAX_MEMBERS) fail(409, "full", `A party holds up to ${MAX_MEMBERS} people, counting open invites.`);
  must(await db.from("vodka_party_invites").upsert({ party_id: party.id, roblox_id: to, invited_by: me }));
  return { ok: true };
}

async function accept(me: number, body: Record<string, unknown>) {
  const partyId = userId(body.partyId);
  const inv = must(await db.from("vodka_party_invites").select("party_id").eq("party_id", partyId).eq("roblox_id", me).maybeSingle());
  if (!inv) fail(404, "no_invite", "That invite is gone. The party may have ended.");
  if (await myParty(me)) fail(409, "in_party", "Leave your current party first.");
  const members = await db.from("vodka_party_members").select("roblox_id", { count: "exact", head: true }).eq("party_id", partyId);
  if ((members.count ?? 0) >= MAX_MEMBERS) fail(409, "full", "That party is full.");
  must(await db.from("vodka_party_members").insert({ party_id: partyId, roblox_id: me }));
  must(await db.from("vodka_party_invites").delete().eq("party_id", partyId).eq("roblox_id", me));
  await system(partyId, `${await nameOf(me)} joined.`);
  return { ok: true };
}

async function decline(me: number, body: Record<string, unknown>) {
  must(await db.from("vodka_party_invites").delete().eq("party_id", userId(body.partyId)).eq("roblox_id", me));
  return { ok: true };
}

async function leave(me: number) {
  const party = await needParty(me);
  if (party.leader === me) {
    // The leader leaving ends the party for everyone.
    must(await db.from("vodka_parties").delete().eq("id", party.id));
  } else {
    must(await db.from("vodka_party_members").delete().eq("party_id", party.id).eq("roblox_id", me));
    await system(party.id, `${await nameOf(me)} left.`);
  }
  return { ok: true };
}

async function kick(me: number, body: Record<string, unknown>) {
  const who = userId(body.robloxId);
  const party = await needParty(me);
  if (party.leader !== me) fail(403, "not_leader", "Only the party leader can remove people.");
  if (who === me) fail(400, "self", "Use Leave to end the party.");
  must(await db.from("vodka_party_members").delete().eq("party_id", party.id).eq("roblox_id", who));
  must(await db.from("vodka_party_invites").delete().eq("party_id", party.id).eq("roblox_id", who));
  await system(party.id, `${await nameOf(who)} was removed.`);
  return { ok: true };
}

async function rateLimit(me: number, partyId: number) {
  const recent = await db
    .from("vodka_party_events")
    .select("id", { count: "exact", head: true })
    .eq("party_id", partyId)
    .eq("sender", me)
    .gt("created_at", new Date(Date.now() - 60_000).toISOString());
  if ((recent.count ?? 0) >= SENDS_PER_MINUTE) fail(429, "slow_down", "You're sending too fast. Wait a moment.");
}

async function send(me: number, body: Record<string, unknown>) {
  const party = await needParty(me);
  const text = String(body.body ?? "").replace(/[\u0000-\u0008\u000b-\u001f\u007f]/g, "").trim();
  if (!text) fail(400, "empty", "Write something first.");
  if (text.length > 1000) fail(400, "too_long", "Messages can be up to 1000 characters.");
  await rateLimit(me, party.id);
  must(await db.from("vodka_party_events").insert({ party_id: party.id, sender: me, kind: "text", body: text }));
  return { ok: true };
}

function launchPayload(raw: unknown) {
  const bad = () => fail(400, "bad_invite", "That game is malformed.");
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

async function launch(me: number, body: Record<string, unknown>) {
  const party = await needParty(me);
  if (party.leader !== me) fail(403, "not_leader", "Only the party leader can call everyone into a game.");
  const payload = launchPayload(body.payload);
  await rateLimit(me, party.id);
  must(await db.from("vodka_party_events").insert({ party_id: party.id, sender: me, kind: "launch", body: "", payload }));
  return { ok: true };
}

async function sync(me: number, body: Record<string, unknown>) {
  const since = typeof body.since === "number" && Number.isSafeInteger(body.since) && body.since >= 0 ? body.since : 0;
  const now = Date.now();
  const invites = must(await db.from("vodka_party_invites").select("party_id, invited_by").eq("roblox_id", me));
  const inviters = invites!.length
    ? must(await db.from("vodka_users").select("roblox_id, display_name").in("roblox_id", invites!.map((i) => i.invited_by)))!
    : [];
  const invitesOut = invites!.map((i) => ({
    partyId: i.party_id,
    from: inviters.find((u) => u.roblox_id === i.invited_by)?.display_name ?? "Someone",
  }));
  const party = await myParty(me);
  if (!party) return { party: null, invites: invitesOut, events: [] };
  const memberRows = must(await db.from("vodka_party_members").select("roblox_id, joined_at").eq("party_id", party.id).order("joined_at"));
  const people = must(await db.from("vodka_users").select("roblox_id, name, display_name, last_seen").in("roblox_id", memberRows!.map((m) => m.roblox_id)));
  const pending = must(await db.from("vodka_party_invites").select("roblox_id").eq("party_id", party.id));
  const events = must(
    await db.from("vodka_party_events").select("id, sender, kind, body, payload, created_at").eq("party_id", party.id).gt("id", since).order("id").limit(300),
  );
  return {
    party: {
      id: party.id,
      leader: party.leader,
      members: memberRows!.map((m) => {
        const p = people!.find((u) => u.roblox_id === m.roblox_id);
        return {
          robloxId: m.roblox_id,
          name: p?.name ?? "",
          displayName: p?.display_name ?? "Someone",
          online: p ? now - new Date(p.last_seen).getTime() < ONLINE_SECONDS * 1000 : false,
        };
      }),
      pending: pending!.map((p) => p.roblox_id),
    },
    invites: invitesOut,
    events: events!.map((e) => ({ id: e.id, sender: e.sender, kind: e.kind, body: e.body, payload: e.payload, createdAt: e.created_at })),
  };
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
    const me = await signedIn(req);
    switch (body.action) {
      case "sync":
        return json(await sync(me, body));
      case "create":
        return json(await create(me));
      case "invite":
        return json(await invite(me, body));
      case "accept":
        return json(await accept(me, body));
      case "decline":
        return json(await decline(me, body));
      case "leave":
        return json(await leave(me));
      case "kick":
        return json(await kick(me, body));
      case "send":
        return json(await send(me, body));
      case "launch":
        return json(await launch(me, body));
    }
    return fail(400, "bad_request", "Unknown action.");
  } catch (e) {
    if (e instanceof Fail) return json({ error: { code: e.code, message: e.message } }, e.status);
    console.error("unhandled", e);
    return json({ error: { code: "server", message: "The party server hit an unexpected problem." } }, 500);
  }
});
