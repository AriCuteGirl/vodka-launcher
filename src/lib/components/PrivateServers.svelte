<script lang="ts">
  import { api } from "$lib/api";
  import { app, refreshRobux, toast } from "$lib/state.svelte";
  import { toAppError, type Game, type Loadable, type OwnedServer, type ServerLists, type ServerOffer } from "$lib/types";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import Skeleton from "./Skeleton.svelte";

  // Roblox's private (VIP) servers for one game: ones you can join, your own to manage, and making a new one.
  let {
    game,
    lists,
    canJoin,
    whyNot = null,
    onjoin,
    onclose,
    onrefresh,
  }: {
    game: Game;
    lists: Loadable<ServerLists>;
    canJoin: boolean;
    whyNot?: string | null;
    onjoin: (accessCode: string) => void;
    onclose: () => void;
    /** Re-reads the game's server lists, after a new private server was made. */
    onrefresh: () => void;
  } = $props();

  let owned = $state<Loadable<OwnedServer[]>>({ status: "loading" });
  let busy = $state<number | null>(null);

  async function loadOwned() {
    owned = { status: "loading" };
    try {
      owned = { status: "ready", data: await api.ownedServers(game.universeId) };
    } catch (e) {
      owned = { status: "error", error: toAppError(e) };
    }
  }
  void loadOwned();

  const joinable = $derived(lists.status === "ready" ? lists.data.private : []);

  async function toggle(s: OwnedServer) {
    busy = s.id;
    try {
      await api.setServerActive(s.id, !s.active);
      if (owned.status === "ready") owned.data = owned.data.map((o) => (o.id === s.id ? { ...o, active: !s.active } : o));
      toast(s.active ? `${s.name} is turned off. Nobody can join it until you turn it back on.` : `${s.name} is open again.`, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    busy = null;
  }

  async function friends(s: OwnedServer) {
    busy = s.id;
    const allow = !s.friendsAllowed;
    try {
      await api.setServerFriends(s.id, allow);
      if (owned.status === "ready") owned.data = owned.data.map((o) => (o.id === s.id ? { ...o, friendsAllowed: allow } : o));
      toast(allow ? `Your friends can now join ${s.name} without an invite.` : `Only people you invite can join ${s.name} now.`, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    busy = null;
  }

  async function copyLink(s: OwnedServer) {
    busy = s.id;
    try {
      const link = await api.serverInviteLink(s.id);
      await navigator.clipboard.writeText(link);
      toast(`Invite link for ${s.name} copied. Anyone with it can join.`, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    busy = null;
  }

  // Making one, in Vodka: the price comes from Roblox's own game page, you confirm it here, and Vodka asks Roblox
  // for exactly that price (Roblox refuses if it changed). If Roblox wants a captcha, its own page finishes the job.
  let offer = $state<Loadable<ServerOffer>>({ status: "loading" });
  let name = $state(`${app.account?.displayName ?? "My"}'s server`.slice(0, 50));
  let agreed = $state(false);
  // Remembered between servers: most people want friends in by default.
  const FRIENDS_KEY = "vodka-ps-friends";
  let letFriends = $state(readFriends());
  function readFriends() {
    try {
      return localStorage.getItem(FRIENDS_KEY) !== "no";
    } catch {
      return true;
    }
  }
  $effect(() => {
    try {
      localStorage.setItem(FRIENDS_KEY, letFriends ? "yes" : "no");
    } catch {
      // It just won't be remembered.
    }
  });
  let creating = $state(false);
  let onRoblox = $state(false);

  async function loadOffer() {
    offer = { status: "loading" };
    try {
      offer = { status: "ready", data: await api.privateServerOffer(game.universeId) };
    } catch (e) {
      offer = { status: "error", error: toAppError(e) };
    }
  }
  void loadOffer();

  const price = $derived(offer.status === "ready" ? offer.data.price : null);
  const free = $derived(price === 0);
  const tooPoor = $derived(price != null && price > 0 && app.robux != null && app.robux < price);
  const ready = $derived(offer.status === "ready" && offer.data.canCreate && price != null && name.trim().length > 0 && (free || agreed) && !tooPoor);

  async function create(e: SubmitEvent) {
    e.preventDefault();
    if (!ready || price == null) return;
    creating = true;
    try {
      const note = await api.createPrivateServer(game.universeId, name.trim(), price, letFriends);
      const who = letFriends && !note ? " Your friends can join it." : "";
      toast(free ? `${name.trim()} is ready. It's free.${who}` : `${name.trim()} is ready. ${price} Robux a month until you cancel.${who}`, "ok", 7000);
      if (note) toast(`The server was made, but: ${note}`, "danger", 9000);
      agreed = false;
      void loadOwned();
      void refreshRobux();
      onrefresh();
    } catch (err) {
      toast(toAppError(err).message, "danger", 9000);
    }
    creating = false;
  }

  async function onRobloxPage() {
    onRoblox = true;
    try {
      await api.openPrivateServerPage(game.placeId, game.name);
    } catch (err) {
      onRoblox = false;
      toast(toAppError(err).message, "danger");
    }
  }
  $effect(() => {
    const off = api.onRobloxPageClosed(() => {
      onRoblox = false;
      void loadOwned();
      void loadOffer();
      onrefresh();
    });
    return () => void off.then((f) => f());
  });

  const date = (iso: string | null) => (iso ? new Date(iso).toLocaleDateString() : null);
</script>

<Modal title="Private servers · {game.name}" {onclose} wide>
  <div class="body">
    <section>
      <h3>Servers you can join</h3>
      {#if lists.status === "loading"}
        <Skeleton height="58px" />
      {:else if lists.status === "error"}
        <p class="err">{lists.error.message}</p>
      {:else if joinable.length === 0}
        <p class="faint">{lists.data.privateError ? `Roblox didn't let Vodka check: ${lists.data.privateError}` : "None yet. Friends can add you to theirs, or make your own below."}</p>
      {:else}
        <ul class="list">
          {#each joinable as p (p.accessCode)}
            <li>
              <span class="lock"><Icon name="lock" size={16} /></span>
              <div class="who"><b>{p.name}</b><small>{p.owner ? `${p.owner}'s · ` : ""}{p.playing}{p.maxPlayers ? ` of ${p.maxPlayers}` : ""} playing</small></div>
              <Button size="sm" variant="primary" disabled={!canJoin} title={whyNot ?? undefined} onclick={() => onjoin(p.accessCode)}>Join</Button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section>
      <h3>Yours</h3>
      {#if owned.status === "loading"}
        <Skeleton height="58px" />
      {:else if owned.status === "error"}
        <p class="err">{owned.error.message} <button class="link" onclick={loadOwned}>Retry</button></p>
      {:else if owned.data.length === 0}
        <p class="faint">You don't own a private server for this game.</p>
      {:else}
        <ul class="list">
          {#each owned.data as s (s.id)}
            <li class:off={!s.active}>
              <span class="lock"><Icon name="star" size={16} /></span>
              <div class="who">
                <b>{s.name}</b>
                <small>
                  {s.active ? "Open" : "Turned off"}{#if date(s.expires)} · {s.willRenew ? "renews" : "ends"} {date(s.expires)}{/if}{#if s.price != null} · {s.price} Robux a month{/if}
                </small>
              </div>
              <button
                class="friends"
                class:on={s.friendsAllowed}
                disabled={busy === s.id || s.friendsAllowed == null}
                aria-pressed={s.friendsAllowed ?? false}
                onclick={() => friends(s)}
                title={s.friendsAllowed == null ? "Roblox didn't say who can join" : s.friendsAllowed ? "Friends can join without an invite. Click to stop." : "Only invited people can join. Click to let friends in."}
              >
                <Icon name="friends" size={14} />{s.friendsAllowed ? "Friends can join" : "Invite only"}
              </button>
              <label class="switch" title={s.active ? "Turn off: nobody can join" : "Turn on"}>
                <input type="checkbox" checked={s.active} disabled={busy === s.id} onchange={() => toggle(s)} />
                <span></span>
              </label>
              <Button size="sm" icon="copy" disabled={busy === s.id} onclick={() => copyLink(s)}>Invite link</Button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="create">
      <h3>Make your own</h3>
      {#if offer.status === "loading"}
        <Skeleton height="96px" />
      {:else if offer.status === "error"}
        <p class="err">{offer.error.message} <button class="link" onclick={loadOffer}>Retry</button></p>
      {:else if owned.status === "ready" && owned.data.length}
        <p class="have">
          <Icon name="check" size={16} />
          <span>You already own a private server for this game: <b>{owned.data.map((o) => o.name).join(", ")}</b>. Manage it under "Yours".</span>
        </p>
      {:else if !offer.data.canCreate}
        <p class="faint">This game doesn't offer private servers, or Roblox doesn't let this account make another one.</p>
      {:else}
        <form class="make" onsubmit={create}>
          <label class="name">
            <span>Name</span>
            <input bind:value={name} maxlength="50" spellcheck="false" autocomplete="off" />
          </label>
          <div class="price">
            <span class="tag" class:free>{free ? "Free" : `${price} Robux a month`}</span>
            {#if !free && app.robux != null}<span class="faint">You have {app.robux.toLocaleString()} Robux</span>{/if}
          </div>
          {#if !free}
            <label class="agree">
              <input type="checkbox" bind:checked={agreed} />
              <span>Pay <b>{price} Robux</b> now. It renews every month at that price until you cancel it on roblox.com.</span>
            </label>
          {/if}
          <label class="agree">
            <input type="checkbox" bind:checked={letFriends} />
            <span>Let my friends join without an invite</span>
          </label>
          {#if tooPoor}<p class="err">You don't have enough Robux for this.</p>{/if}
          <div class="go">
            <Button type="submit" variant="primary" icon="plus" busy={creating} disabled={!ready}>
              {free ? "Create free server" : `Create for ${price} Robux`}
            </Button>
          </div>
        </form>
      {/if}
      {#if !(owned.status === "ready" && owned.data.length)}
      <p class="fallback faint">
        {#if onRoblox}Finish in the Roblox window, then close it.{:else}Roblox asking for a captcha? <button class="link" onclick={onRobloxPage}>Do it on Roblox</button>{/if}
      </p>
      {/if}
    </section>
  </div>
  {#snippet actions()}
    <Button onclick={onclose}>Close</Button>
  {/snippet}
</Modal>

<style>
  .body {
    display: grid;
    gap: var(--s-5);
  }
  section {
    display: grid;
    gap: var(--s-2);
  }
  h3 {
    font-size: var(--fs-xs);
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-2);
  }
  .list {
    display: grid;
    gap: var(--s-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .list li {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-2) var(--s-2) var(--s-2) var(--s-3);
    border-radius: var(--r-md);
    background: var(--glass);
    border: 1px solid var(--line);
  }
  .list li.off {
    opacity: 0.6;
  }
  .lock {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: var(--r-sm);
    background: var(--glass-strong);
    color: var(--frost);
  }
  .who {
    flex: 1;
    display: grid;
    min-width: 0;
    line-height: 1.3;
  }
  .who b {
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who small {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .friends {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 10px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    background: var(--glass);
    color: var(--text-2);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
  }
  .friends.on {
    border-color: rgba(116, 230, 180, 0.45);
    background: rgba(116, 230, 180, 0.12);
    color: var(--ok);
  }
  .friends:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .switch {
    position: relative;
    width: 38px;
    height: 22px;
    flex: none;
    cursor: pointer;
  }
  .switch input {
    position: absolute;
    opacity: 0;
    inset: 0;
    margin: 0;
    cursor: pointer;
  }
  .switch span {
    position: absolute;
    inset: 0;
    border-radius: var(--r-full);
    background: var(--glass-strong);
    border: 1px solid var(--line-strong);
    transition: background var(--fast) var(--ease);
  }
  .switch span::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-1);
    transition: transform var(--fast) var(--ease);
  }
  .switch input:checked + span {
    background: var(--ok);
  }
  .switch input:checked + span::after {
    transform: translateX(16px);
    background: #fff;
  }
  .switch input:focus-visible + span {
    outline: 2px solid var(--frost);
    outline-offset: 2px;
  }
  .make {
    display: grid;
    gap: var(--s-3);
    padding: var(--s-4);
    border-radius: var(--r-lg);
    border: 1px solid var(--line-strong);
    background: var(--glass);
  }
  .name {
    display: grid;
    gap: 4px;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .name input {
    height: 36px;
    padding: 0 var(--s-3);
    border-radius: var(--r-sm);
    border: 1px solid var(--line-strong);
    background: rgba(0, 0, 0, 0.25);
    color: var(--text-0);
    font-size: var(--fs-sm);
    user-select: text;
  }
  .name input:focus {
    outline: none;
    border-color: var(--frost);
  }
  .price {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    font-size: var(--fs-xs);
  }
  .tag {
    padding: 4px 12px;
    border-radius: var(--r-full);
    background: var(--glass-strong);
    color: var(--frost);
    font-size: var(--fs-sm);
    font-weight: 700;
  }
  .tag.free {
    color: var(--ok);
    background: rgba(116, 230, 180, 0.12);
  }
  .agree {
    display: flex;
    align-items: flex-start;
    gap: var(--s-2);
    font-size: var(--fs-sm);
    color: var(--text-1);
    cursor: pointer;
  }
  .agree input {
    margin-top: 3px;
  }
  .go {
    display: flex;
    justify-content: flex-end;
  }
  .have {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: var(--s-3) var(--s-4);
    border-radius: var(--r-lg);
    border: 1px solid rgba(116, 230, 180, 0.35);
    background: rgba(116, 230, 180, 0.08);
    color: var(--ok);
    font-size: var(--fs-sm);
  }
  .have span {
    color: var(--text-1);
  }
  .have b {
    color: var(--text-0);
  }
  .fallback {
    font-size: var(--fs-xs);
  }
  .err {
    color: var(--danger);
    font-size: var(--fs-sm);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--frost);
    font-weight: 600;
    cursor: pointer;
  }
</style>
