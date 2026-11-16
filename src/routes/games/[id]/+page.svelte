<script lang="ts">
  import { page } from "$app/state";
  import { api } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import GamePage from "$lib/components/GamePage.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import PrivateServers from "$lib/components/PrivateServers.svelte";
  import ServerBrowser, { type BrowserServer } from "$lib/components/ServerBrowser.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { compact } from "$lib/format";
  import { app, launchMany, loadHome, openLaunchDialog, phaseOf, play, signIn, toast } from "$lib/state.svelte";
  import { toAppError, type Game, type GameDetails, type GameStanding, type GroupInfo, type Loadable, type ServerLists, type Target } from "$lib/types";

  // A Roblox game's page: Play at the top, then pictures, servers and what the game is about.
  const universeId = $derived(Number(page.params.id));

  let details = $state<Loadable<GameDetails>>({ status: "loading" });
  let standing = $state<GameStanding | null>(null);
  let standingBusy = $state(false);
  let servers = $state<Loadable<ServerLists>>({ status: "loading" });

  async function loadServers(placeId: number) {
    servers = { status: "loading" };
    try {
      servers = { status: "ready", data: await api.listServers(placeId) };
    } catch (e) {
      servers = { status: "error", error: toAppError(e) };
    }
  }

  $effect(() => {
    const id = universeId;
    details = { status: "loading" };
    standing = null;
    servers = { status: "loading" };
    api
      .gameDetails(id)
      .then((d) => {
        details = { status: "ready", data: d };
        if (d.rootPlaceId) void loadServers(d.rootPlaceId);
      })
      .catch((e) => (details = { status: "error", error: toAppError(e) }));
    if (app.account) api.gameStanding(id).then((s) => (standing = s)).catch(() => {});
  });

  const d = $derived(details.status === "ready" ? details.data : null);
  const game = $derived<Game | null>(d ? { universeId: d.universeId, placeId: d.rootPlaceId, name: d.name, playing: d.playing, iconUrl: d.iconUrl } : null);
  const phase = $derived(game ? phaseOf(game) : "idle");
  const canPlay = $derived(!!app.mocktail?.installed && !!app.account);
  const likeShare = $derived(
    standing?.upVotes != null && standing.downVotes != null && standing.upVotes + standing.downVotes > 0
      ? Math.round((standing.upVotes / (standing.upVotes + standing.downVotes)) * 100)
      : null,
  );

  async function change(run: () => Promise<GameStanding>, done: string) {
    if (standingBusy) return;
    standingBusy = true;
    try {
      standing = await run();
      toast(done, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    standingBusy = false;
  }
  function favorite() {
    if (!standing || !d) return;
    const on = !standing.favorited;
    void change(() => api.setFavorite(universeId, on), on ? `Added ${d.name} to favorites.` : `Removed ${d.name} from favorites.`).then(() => void loadHome());
  }
  function vote(like: boolean) {
    if (!d) return;
    void change(() => api.setVote(universeId, like), like ? `You liked ${d.name}.` : `You disliked ${d.name}.`);
  }
  // The group behind the game: join it from here. Roblox runs a human check on group joins; Vodka lets Roblox's
  // own page do that in a hidden window, and only shows it if Roblox wants a puzzle solved.
  let group = $state<GroupInfo | null>(null);
  let joining = $state(false);
  $effect(() => {
    const id = d?.creatorType === "Group" ? d.creatorId : null;
    group = null;
    if (id) api.robloxGroup(id).then((g) => (group = g)).catch(() => {});
  });
  async function joinGroup() {
    if (!group) return;
    joining = true;
    try {
      group = await api.joinRobloxGroup(group.id, group.name);
      if (group.myRole) toast(`You joined ${group.name}.`, "ok");
      else if (!group.publicEntry) toast(`Asked to join ${group.name}. The group approves new members.`, "ok");
      else toast(`Didn't join ${group.name}. If Roblox showed a puzzle, finish it, or try again.`, "danger");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    joining = false;
  }
  $effect(() => {
    const off = api.onRobloxPageClosed(() => {
      if (group) api.robloxGroup(group.id).then((g) => (group = g)).catch(() => {});
    });
    return () => void off.then((f) => f());
  });
  function join(target: Target) {
    if (!game || !app.account) return;
    void launchMany(game, app.batch.length ? app.batch : [app.account.id], target);
  }

  const lists = $derived(servers.status === "ready" ? servers.data : null);
  const publicServers = $derived<Loadable<BrowserServer[]>>(
    servers.status === "ready" ? { status: "ready", data: servers.data.public } : servers,
  );
  const whyNot = $derived(!app.account ? "Sign in first" : !app.mocktail?.installed ? "Install Mocktail first" : null);
  let showServers = $state(false);
  let showPrivate = $state(false);
  const serverCount = $derived(lists ? lists.public.length : null);
  const about = $derived.by(() => {
    if (!d) return [];
    const out: { label: string; value: string }[] = [];
    if (d.playing != null) out.push({ label: "Playing", value: compact(d.playing) });
    if (d.visits != null) out.push({ label: "Visits", value: compact(d.visits) });
    if (d.favorites != null) out.push({ label: "Favorites", value: compact(d.favorites) });
    if (d.maxPlayers) out.push({ label: "Server size", value: String(d.maxPlayers) });
    if (d.genre) out.push({ label: "Genre", value: d.genre });
    if (d.updated) out.push({ label: "Updated", value: new Date(d.updated).toLocaleDateString() });
    return out;
  });
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
    title={d?.name ?? "Loading…"}
    banner={d?.bannerUrl}
    icon={d?.iconUrl}
    creator={d?.creator}
    verified={d?.creatorVerified}
    subtitle={d?.genre}
    media={d?.gallery ?? []}
    loading={!d}
  >
    {#snippet actions()}
      {#if !app.account}
        <Button variant="primary" size="lg" busy={app.signingIn} onclick={signIn}>Sign in to play</Button>
      {:else if phase === "playing"}
        <Button variant="primary" size="lg" disabled>Playing now</Button>
      {:else}
        <Button variant="primary" size="lg" icon="play" disabled={!game || !canPlay} busy={phase === "starting"} onclick={() => game && play(game)}>
          {phase === "starting" ? "Starting" : "Play"}
        </Button>
      {/if}
      <Button size="lg" icon="games" disabled={!game} onclick={() => (showServers = true)}>Servers{#if serverCount != null}<small class="n">{serverCount}</small>{/if}</Button>
      <Button size="lg" icon="lock" disabled={!game || !app.account} onclick={() => (showPrivate = true)} title={app.account ? undefined : "Sign in first"}>Private servers</Button>
      {#if app.account}
        <Button size="lg" icon="players" disabled={!game} onclick={() => game && openLaunchDialog(game)} title="Play on several accounts at once">Accounts</Button>
      {/if}
    {/snippet}

    {#snippet pills()}
      <button class="pill" class:liked={standing?.vote === true} disabled={!standing?.canVote || standingBusy} aria-pressed={standing?.vote === true} onclick={() => vote(true)} title={standing?.whyNot ?? "Like"}>
        <Icon name="thumb" size={16} filled={standing?.vote === true} />{likeShare != null ? `${likeShare}%` : "Like"}
      </button>
      <button class="pill" class:disliked={standing?.vote === false} disabled={!standing?.canVote || standingBusy} aria-pressed={standing?.vote === false} onclick={() => vote(false)} aria-label="Dislike" title={standing?.whyNot ?? "Dislike"}>
        <span class="down"><Icon name="thumb" size={16} filled={standing?.vote === false} /></span>
      </button>
      {#if d?.playing != null}<span class="pill static"><Icon name="players" size={16} />{compact(d.playing)} active</span>{/if}
      <button class="pill" class:on={standing?.favorited} disabled={!standing || standingBusy} aria-pressed={standing?.favorited ?? false} onclick={favorite}>
        <Icon name="star" size={16} filled={standing?.favorited} />{standing?.favorited ? "Favorited" : "Favorite"}
      </button>
      {#if group}
        {#if group.myRole}
          <span class="pill static on" title="You're in {group.name} ({compact(group.members)} members)"><Icon name="check" size={16} />In {group.name}<small>{group.myRole}</small></span>
        {:else}
          <button class="pill" disabled={joining || !app.account} onclick={joinGroup} title="{compact(group.members)} members{group.publicEntry ? '' : ' · the group approves new members'}">
            <Icon name="friends" size={16} />{joining ? "Joining" : group.publicEntry ? "Join" : "Ask to join"} {group.name}
          </button>
        {/if}
      {/if}
    {/snippet}


    <section class="section">
      <h2>About</h2>
      {#if !d}
        <Skeleton height="80px" />
      {:else}
        {#if about.length}
          <dl class="stats">{#each about as s (s.label)}<div><dt>{s.label}</dt><dd>{s.value}</dd></div>{/each}</dl>
        {/if}
        {#if d.description}<p class="desc selectable">{d.description}</p>{/if}
      {/if}
    </section>
  </GamePage>
{/if}

{#if showServers && game}
  <Modal title="Servers · {game.name}" wide onclose={() => (showServers = false)}>
    <ServerBrowser
      servers={publicServers}
      canJoin={canPlay}
      {whyNot}
      onrefresh={() => game && loadServers(game.placeId)}
      onjoin={(srv) => {
        showServers = false;
        if (game) join({ kind: "server", placeId: game.placeId, gameId: srv.id });
      }}
    />
  </Modal>
{/if}

{#if showPrivate && game}
  <PrivateServers
    {game}
    lists={servers}
    canJoin={canPlay}
    {whyNot}
    onclose={() => (showPrivate = false)}
    onrefresh={() => game && loadServers(game.placeId)}
    onjoin={(code) => {
      showPrivate = false;
      join({ kind: "private", accessCode: code });
    }}
  />
{/if}

<style>
  .n {
    margin-left: 4px;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .pad {
    padding: var(--s-4) var(--s-4) 0 0;
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
