<script lang="ts">
  import { page } from "$app/state";
  import { api } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import GamePage from "$lib/components/GamePage.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import ServerBrowser, { type BrowserServer } from "$lib/components/ServerBrowser.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { compact } from "$lib/format";
  import { luduvo, luduvoSignIn, playLuduvo, playLuduvoPrivate, playLuduvoServer, startLuduvo, stopLuduvo } from "$lib/luduvo.svelte";
  import { toast } from "$lib/state.svelte";
  import { toAppError, type Loadable, type LuduvoDetails, type LuduvoRating, type LuduvoServer } from "$lib/types";
  import { onMount } from "svelte";

  // A Luduvo game's page: Play at the top, then its servers (with who's in them) and what it's about.
  onMount(startLuduvo);
  const placeId = $derived(Number(page.params.id));

  let details = $state<Loadable<LuduvoDetails>>({ status: "loading" });
  let rating = $state<LuduvoRating | null>(null);
  let voting = $state(false);
  let joiningGroup = $state(false);
  let servers = $state<Loadable<LuduvoServer[]>>({ status: "loading" });

  async function loadServers(id: number) {
    servers = { status: "loading" };
    try {
      servers = { status: "ready", data: (await api.luduvoServers(id)).sort((a, b) => b.playing - a.playing) };
    } catch (e) {
      servers = { status: "error", error: toAppError(e) };
    }
  }

  $effect(() => {
    const id = placeId;
    details = { status: "loading" };
    rating = null;
    api
      .luduvoDetails(id)
      .then((d) => (details = { status: "ready", data: d }))
      .catch((e) => (details = { status: "error", error: toAppError(e) }));
    api.luduvoRating(id).then((r) => (rating = r)).catch(() => {});
    void loadServers(id);
  });

  const d = $derived(details.status === "ready" ? details.data : null);
  const p = $derived(d?.place ?? null);
  const canPlay = $derived(!!luduvo.status?.installed && luduvo.status.supported);
  const playingThis = $derived(luduvo.playing?.placeId === placeId);
  const ageLabel: Record<string, string> = { all_ages: "All ages", ages_9_plus: "Ages 9+", ages_13_plus: "Ages 13+", ages_17_plus: "Ages 17+" };
  const likeShare = $derived.by(() => {
    const likes = rating?.likes ?? p?.likes ?? 0;
    const dislikes = rating?.dislikes ?? p?.dislikes ?? 0;
    return likes + dislikes > 0 ? Math.round((likes / (likes + dislikes)) * 100) : null;
  });
  const about = $derived.by(() => {
    if (!d || !p) return [];
    const out = [
      { label: "Playing", value: p.maxPlayers ? `${p.playing} / ${p.maxPlayers}` : String(p.playing) },
      { label: "Visits", value: compact(p.visits) },
    ];
    if (d.createdAt) out.push({ label: "Created", value: new Date(d.createdAt * 1000).toLocaleDateString() });
    if (d.updatedAt) out.push({ label: "Updated", value: new Date(d.updatedAt * 1000).toLocaleDateString() });
    return out;
  });

  const browserServers = $derived<Loadable<BrowserServer[]>>(servers);
  const whyNot = $derived(
    !luduvo.user ? "Sign in to Luduvo first" : !canPlay ? "Install Luduvo first" : luduvo.playing ? `Close ${luduvo.playing.title} first` : null,
  );
  let showServers = $state(false);
  let showPrivate = $state(false);
  let code = $state("");
  let joiningPrivate = $state(false);
  async function joinPrivate(e: SubmitEvent) {
    e.preventDefault();
    if (!p || !code.trim()) return;
    joiningPrivate = true;
    if (await playLuduvoPrivate(p, code)) {
      code = "";
      showPrivate = false;
    }
    joiningPrivate = false;
  }

  async function vote(value: number) {
    if (!luduvo.user) {
      toast("Sign in to Luduvo to rate games.", "info");
      return;
    }
    if (voting || !rating) return;
    voting = true;
    // Pressing your current vote again takes it back, like on luduvo.com.
    const next = rating.vote === value ? 0 : value;
    try {
      rating = await api.luduvoRate(placeId, next);
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    voting = false;
  }

  async function joinGroup() {
    if (!d?.groupId) return;
    if (!luduvo.user) {
      toast("Sign in to Luduvo to join groups.", "info");
      return;
    }
    joiningGroup = true;
    try {
      await api.luduvoJoinGroup(d.groupId);
      toast(`You joined ${d.groupName ?? "the group"}.`, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    joiningGroup = false;
  }
</script>

{#if details.status === "error"}
  <div class="pad">
    <StateCard icon="alert" tone="danger" title="Couldn't open this game">
      {details.error.message}
      {#snippet actions()}<Button onclick={() => history.back()}>Back</Button>{/snippet}
    </StateCard>
  </div>
{:else}
  <GamePage
    title={p?.title ?? "Loading…"}
    banner={p?.bannerUrl}
    icon={p?.iconUrl}
    creator={d?.groupName ?? p?.creator}
    subtitle={d?.ageRating ? (ageLabel[d.ageRating] ?? d.ageRating) : null}
    media={p?.bannerUrl ? [p.bannerUrl] : []}
    loading={!d}
  >
    {#snippet actions()}
      {#if !luduvo.user}
        <Button variant="primary" size="lg" busy={luduvo.signingIn} onclick={luduvoSignIn}>Sign in to play</Button>
      {:else if playingThis}
        <Button variant="primary" size="lg" disabled>Playing now</Button>
        <Button size="lg" icon="close" onclick={stopLuduvo}>Close game</Button>
      {:else}
        <Button
          variant="primary"
          size="lg"
          icon="play"
          disabled={!p || !canPlay || !!luduvo.playing}
          busy={luduvo.starting === placeId}
          onclick={() => p && playLuduvo(p)}
          title={luduvo.playing ? `Close ${luduvo.playing.title} first` : undefined}
        >
          {luduvo.starting === placeId ? "Starting" : "Play"}
        </Button>
      {/if}
      <Button size="lg" icon="games" onclick={() => (showServers = true)}>Servers{#if servers.status === "ready"}<small class="n">{servers.data.length}</small>{/if}</Button>
      <Button size="lg" icon="lock" onclick={() => (showPrivate = true)}>Private server</Button>
    {/snippet}

    {#snippet pills()}
      <button class="pill" class:liked={rating?.vote === 1} disabled={!rating || voting} aria-pressed={rating?.vote === 1} onclick={() => vote(1)}>
        <Icon name="thumb" size={16} filled={rating?.vote === 1} />{likeShare != null ? `${likeShare}%` : "Like"}
      </button>
      <button class="pill" class:disliked={rating?.vote === -1} disabled={!rating || voting} aria-pressed={rating?.vote === -1} onclick={() => vote(-1)} aria-label="Dislike">
        <span class="down"><Icon name="thumb" size={16} filled={rating?.vote === -1} /></span>
      </button>
      {#if p}<span class="pill static"><Icon name="players" size={16} />{p.playing} active</span>{/if}
      {#if d?.groupId}
        <button class="pill" disabled={joiningGroup} onclick={joinGroup}><Icon name="friends" size={16} />Join {d.groupName ?? "group"}</button>
      {/if}
      {#each d?.contentWarnings ?? [] as w (w)}<span class="pill static warn">{w}</span>{/each}
    {/snippet}


    <section class="section">
      <h2>About</h2>
      {#if !d || !p}
        <Skeleton height="80px" />
      {:else}
        <dl class="stats">{#each about as s (s.label)}<div><dt>{s.label}</dt><dd>{s.value}</dd></div>{/each}</dl>
        {#if p.description}<p class="desc selectable">{p.description}</p>{/if}
      {/if}
    </section>
  </GamePage>
{/if}

{#if showServers && p}
  <Modal title="Servers · {p.title}" wide onclose={() => (showServers = false)}>
    <ServerBrowser
      servers={browserServers}
      canJoin={!whyNot}
      {whyNot}
      onrefresh={() => loadServers(placeId)}
      onjoin={(srv) => {
        showServers = false;
        if (p) void playLuduvoServer(p, srv.id);
      }}
    />
  </Modal>
{/if}

{#if showPrivate && p}
  <Modal title="Private server · {p.title}" onclose={() => (showPrivate = false)}>
    <form class="code" onsubmit={joinPrivate}>
      <Icon name="lock" size={18} />
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={code} placeholder="Paste an invite code or link" spellcheck="false" autocomplete="off" maxlength="300" aria-label="Invite code" autofocus />
      <Button type="submit" variant="primary" size="sm" busy={joiningPrivate} disabled={!code.trim() || !!whyNot} title={whyNot ?? undefined}>Join</Button>
    </form>
    <p class="faint hint">
      Luduvo private servers are made in the game itself, and the owner shares an invite code. Paste it here to join. Luduvo doesn't list
      them, so there's nothing to browse.
    </p>
    {#snippet actions()}<Button onclick={() => (showPrivate = false)}>Close</Button>{/snippet}
  </Modal>
{/if}

<style>
  .pad {
    padding: var(--s-4) var(--s-4) 0 0;
  }
  .n {
    margin-left: 4px;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .hint {
    margin-top: var(--s-3);
  }
  .code {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: 6px 6px 6px var(--s-4);
    border-radius: var(--r-lg);
    border: 1px solid var(--line-strong);
    background: var(--glass);
    color: var(--text-2);
  }
  .code:focus-within {
    border-color: var(--frost);
  }
  .code input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text-0);
    font-size: var(--fs-sm);
    outline: none;
    user-select: text;
  }
  .warn {
    text-transform: capitalize;
    color: var(--warn);
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: var(--s-2);
    margin: 0;
  }
  .stats div {
    padding: var(--s-3);
    border-radius: var(--r-md);
    background: var(--glass);
    border: 1px solid var(--line);
  }
  dt {
    font-size: 0.6875rem;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  dd {
    margin: 0;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  .desc {
    font-size: var(--fs-sm);
    color: var(--text-1);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
