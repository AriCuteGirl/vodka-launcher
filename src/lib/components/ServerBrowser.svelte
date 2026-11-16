<script lang="ts" module>
  /** One server, from either game. */
  export interface BrowserServer {
    id: string;
    playing: number;
    maxPlayers: number;
    ping?: number | null;
    players?: { id: number; name: string; avatarUrl: string | null }[];
  }
</script>

<script lang="ts">
  import type { AppError, Loadable } from "$lib/types";
  import Avatar from "./Avatar.svelte";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import Skeleton from "./Skeleton.svelte";
  import StateCard from "./StateCard.svelte";

  // The Servers pop-out, shared by Roblox and Luduvo: quick joins on top, then cards you can sort.
  let {
    servers,
    canJoin,
    whyNot = null,
    onjoin,
    onrefresh,
  }: {
    servers: Loadable<BrowserServer[]>;
    canJoin: boolean;
    /** Why joining is off right now, for the buttons' tooltip. */
    whyNot?: string | null;
    onjoin: (server: BrowserServer) => void;
    onrefresh: () => void;
  } = $props();

  type Sort = "fewest" | "most" | "ping";
  let sort = $state<Sort>("most");
  let hideFull = $state(true);

  const list = $derived(servers.status === "ready" ? servers.data : []);
  const full = (s: BrowserServer) => s.maxPlayers > 0 && s.playing >= s.maxPlayers;
  const hasPing = $derived(list.some((s) => s.ping != null));
  const open = $derived(list.filter((s) => !full(s)));
  const shown = $derived.by(() => {
    const rows = hideFull ? open : list;
    const by: Record<Sort, (a: BrowserServer, b: BrowserServer) => number> = {
      fewest: (a, b) => a.playing - b.playing,
      most: (a, b) => b.playing - a.playing,
      ping: (a, b) => (a.ping ?? 9999) - (b.ping ?? 9999),
    };
    return [...rows].sort(by[sort]);
  });
  // The quick joins skip empty servers when there's a choice: nobody wants to land alone.
  const fewest = $derived([...open].sort((a, b) => a.playing - b.playing).find((s) => s.playing > 0) ?? open[0] ?? null);
  const busiest = $derived([...open].sort((a, b) => b.playing - a.playing)[0] ?? null);
  const totalPlayers = $derived(list.reduce((n, s) => n + s.playing, 0));

  function fill(s: BrowserServer) {
    return s.maxPlayers ? Math.min(100, (s.playing / s.maxPlayers) * 100) : 0;
  }
  function mood(s: BrowserServer): { label: string; tone: string } {
    const f = fill(s);
    if (full(s)) return { label: "Full", tone: "full" };
    if (f >= 75) return { label: "Almost full", tone: "busy" };
    if (f >= 30) return { label: "Filling up", tone: "mid" };
    return { label: s.playing ? "Quiet" : "Empty", tone: "quiet" };
  }
  function errorOf(l: Loadable<BrowserServer[]>): AppError | null {
    return l.status === "error" ? l.error : null;
  }
</script>

<section class="browser" aria-label="Servers">
    {#if servers.status === "loading"}
      <div class="quick"><Skeleton height="64px" /><Skeleton height="64px" /></div>
      <div class="grid">{#each Array(6) as _, i (i)}<Skeleton height="132px" radius="var(--r-lg)" />{/each}</div>
    {:else if errorOf(servers)}
      <StateCard icon="alert" tone="danger" title="Couldn't list servers">
        {errorOf(servers)?.message}
        {#snippet actions()}<Button icon="refresh" onclick={onrefresh}>Retry</Button>{/snippet}
      </StateCard>
    {:else if list.length === 0}
      <StateCard icon="players" title="No servers running">Nobody's playing right now. Press Play and a fresh server starts for you.</StateCard>
    {:else}
      <div class="quick">
        <button class="qj" disabled={!fewest || !canJoin} title={whyNot ?? undefined} onclick={() => fewest && onjoin(fewest)}>
          <span class="qi"><Icon name="players" size={18} /></span>
          <span><b>Join fewest players</b><small>{fewest ? `${fewest.playing} of ${fewest.maxPlayers || "?"} in it` : "Every server is full"}</small></span>
        </button>
        <button class="qj" disabled={!busiest || !canJoin} title={whyNot ?? undefined} onclick={() => busiest && onjoin(busiest)}>
          <span class="qi hot"><Icon name="friends" size={18} /></span>
          <span><b>Join busiest</b><small>{busiest ? `${busiest.playing} of ${busiest.maxPlayers || "?"}, still has room` : "Every server is full"}</small></span>
        </button>
      </div>

      <div class="bar">
        <span class="faint">{list.length} server{list.length === 1 ? "" : "s"} · {totalPlayers} playing</span>
        <button class="refresh" onclick={onrefresh} aria-label="Refresh servers" title="Refresh"><Icon name="refresh" size={15} /></button>
        <label class="check"><input type="checkbox" bind:checked={hideFull} />Hide full</label>
        <div class="sort" role="radiogroup" aria-label="Sort servers">
          <button role="radio" aria-checked={sort === "most"} class:on={sort === "most"} onclick={() => (sort = "most")}>Most players</button>
          <button role="radio" aria-checked={sort === "fewest"} class:on={sort === "fewest"} onclick={() => (sort = "fewest")}>Fewest</button>
          {#if hasPing}<button role="radio" aria-checked={sort === "ping"} class:on={sort === "ping"} onclick={() => (sort = "ping")}>Best ping</button>{/if}
        </div>
      </div>

      {#if shown.length === 0}
        <p class="faint">Every server is full right now. Untick "Hide full" to see them, or press Play to get a new one.</p>
      {:else}
        <ul class="grid">
          {#each shown.slice(0, 60) as s (s.id)}
            {@const m = mood(s)}
            <li class="card {m.tone}">
              <div class="top">
                <span class="count"><b>{s.playing}</b>/{s.maxPlayers || "?"}</span>
                <span class="mood">{m.label}</span>
              </div>
              <span class="meter" aria-hidden="true"><i style:width="{fill(s)}%"></i></span>
              <div class="bottom">
                {#if s.players?.length}
                  <div class="faces">
                    {#each s.players.slice(0, 5) as p (p.id)}<span title={p.name}><Avatar name={p.name} src={p.avatarUrl} size={22} /></span>{/each}
                    {#if s.players.length > 5}<em>+{s.players.length - 5}</em>{/if}
                  </div>
                {:else if s.ping != null}
                  <span class="ping" class:good={s.ping < 80} class:bad={s.ping >= 160}><i></i>{s.ping} ms</span>
                {:else}
                  <span></span>
                {/if}
                <Button size="sm" variant={full(s) ? "glass" : "primary"} disabled={full(s) || !canJoin} title={whyNot ?? undefined} onclick={() => onjoin(s)}>
                  {full(s) ? "Full" : "Join"}
                </Button>
              </div>
            </li>
          {/each}
        </ul>
        {#if shown.length > 60}<p class="faint">Showing the first 60 of {shown.length}.</p>{/if}
      {/if}
    {/if}
</section>

<style>
  .sort {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r-full);
    background: var(--glass);
    border: 1px solid var(--line);
  }
  .sort button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    border: 0;
    border-radius: var(--r-full);
    background: none;
    color: var(--text-1);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
  }
  .sort button.on {
    background: var(--glass-strong);
    color: var(--frost);
  }
  .browser {
    display: grid;
    gap: var(--s-3);
  }
  .refresh {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--glass);
    color: var(--text-1);
    cursor: pointer;
  }
  .refresh:hover {
    color: var(--text-0);
    background: var(--glass-hover);
  }
  .quick {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: var(--s-3);
  }
  .qj {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4);
    border-radius: var(--r-lg);
    border: 1px solid var(--line-strong);
    background: linear-gradient(135deg, var(--glass-strong), var(--glass));
    text-align: left;
    cursor: pointer;
    transition:
      border-color var(--fast) var(--ease),
      transform var(--fast) var(--ease);
  }
  .qj:hover:not(:disabled) {
    border-color: var(--frost);
    transform: translateY(-1px);
  }
  .qj:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .qj b {
    display: block;
    font-size: var(--fs-sm);
  }
  .qj small {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .qi {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    flex: none;
    border-radius: var(--r-md);
    background: rgba(116, 230, 180, 0.14);
    color: var(--ok);
  }
  .qi.hot {
    background: var(--glass-strong);
    color: var(--frost);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-3);
    font-size: var(--fs-xs);
  }
  .bar .faint {
    margin-right: auto;
  }
  .check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-1);
    cursor: pointer;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--s-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .card {
    display: grid;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4);
    border-radius: var(--r-lg);
    background: var(--glass);
    border: 1px solid var(--line);
    --tone: var(--ok);
  }
  .card.mid {
    --tone: var(--frost-strong);
  }
  .card.busy {
    --tone: var(--warn);
  }
  .card.full {
    --tone: var(--danger);
    opacity: 0.65;
  }
  .card:hover {
    border-color: var(--line-strong);
  }
  .top,
  .bottom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-2);
  }
  .count {
    font-size: var(--fs-sm);
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
  .count b {
    font-size: 1.5rem;
    color: var(--text-0);
  }
  .mood {
    padding: 2px 8px;
    border-radius: var(--r-full);
    font-size: 0.6875rem;
    font-weight: 650;
    color: var(--tone);
    background: color-mix(in srgb, var(--tone) 14%, transparent);
  }
  .meter {
    height: 6px;
    border-radius: var(--r-full);
    background: var(--glass-strong);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--tone);
  }
  .faces {
    display: flex;
    align-items: center;
    gap: 3px;
    min-width: 0;
  }
  .faces em {
    font-style: normal;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .ping {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--text-1);
    font-variant-numeric: tabular-nums;
  }
  .ping i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--warn);
  }
  .ping.good i {
    background: var(--ok);
  }
  .ping.bad i {
    background: var(--danger);
  }
</style>
