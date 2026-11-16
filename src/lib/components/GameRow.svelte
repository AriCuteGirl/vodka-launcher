<script lang="ts">
  import { matches } from "$lib/format";
  import { goto } from "$app/navigation";
  import { app, loadHome, phaseOf, play, signIn } from "$lib/state.svelte";
  import type { Game, Loadable } from "$lib/types";
  import Button from "./Button.svelte";
  import GameTile from "./GameTile.svelte";
  import Icon, { type IconName } from "./Icon.svelte";
  import Skeleton from "./Skeleton.svelte";
  import StateCard from "./StateCard.svelte";

  let {
    title,
    icon,
    row,
    emptyTitle,
    emptyText,
    filter = true,
    wrap = false,
    onretry = loadHome,
  }: {
    /** Apply the title strip's search box to this row. */
    filter?: boolean;
    /** Lay tiles out as a wrapping grid instead of one scrolling line. */
    wrap?: boolean;
    onretry?: () => void;
    title: string; icon: IconName; row: Loadable<Game[]>; emptyTitle: string; emptyText: string } =
    $props();

  const visible = $derived(row.status === "ready" ? row.data.filter((g) => !filter || matches(g.name, app.search)) : []);
  const canPlay = $derived(!!app.mocktail?.installed);

  let scroller = $state<HTMLElement>();
  const nudge = (dir: number) => scroller?.scrollBy({ left: dir * scroller.clientWidth * 0.8, behavior: "smooth" });
</script>

<section aria-labelledby="row-{title}">
  <header>
    <Icon name={icon} size={18} />
    <h2 id="row-{title}">{title}</h2>
    {#if row.status === "ready" && row.data.length}
      <span class="count">{visible.length}</span>
      <span class="arrows" class:hidden={wrap}>
        <button onclick={() => nudge(-1)} aria-label="Scroll {title} left"><Icon name="back" size={15} /></button>
        <button onclick={() => nudge(1)} aria-label="Scroll {title} right"><Icon name="forward" size={15} /></button>
      </span>
    {/if}
  </header>

  {#if row.status === "loading"}
    <div class="grid" class:wrap aria-busy="true" aria-label="Loading {title}">
      {#each Array(6) as _, i (i)}
        <div class="ghost">
          <Skeleton height="auto" radius="var(--r-lg)" />
          <Skeleton width="75%" height="12px" />
          <Skeleton width="45%" height="10px" />
        </div>
      {/each}
    </div>
  {:else if row.status === "error"}
    {#if row.error.kind === "rate_limited"}
      <StateCard icon="clock" tone="warn" title="Roblox asked us to slow down">
        Too many requests in a short time. Wait a minute, then retry.
        {#snippet actions()}<Button icon="refresh" onclick={onretry}>Retry</Button>{/snippet}
      </StateCard>
    {:else if row.error.kind === "offline"}
      <StateCard icon="wifiOff" tone="warn" title="Can't reach Roblox">
        Check your internet connection. Your games will show up once you're back online.
        {#snippet actions()}<Button icon="refresh" onclick={onretry}>Retry</Button>{/snippet}
      </StateCard>
    {:else if row.error.kind === "session_expired" || row.error.kind === "not_signed_in"}
      <StateCard icon="lock" tone="warn" title="Your Roblox session ended">
        Roblox signed this session out. Sign in again to see your games.
        {#snippet actions()}<Button variant="primary" busy={app.signingIn} onclick={signIn}>Sign in</Button>{/snippet}
      </StateCard>
    {:else}
      <StateCard icon="alert" tone="danger" title="Couldn't load {title.toLowerCase()}">
        {row.error.message}
        {#snippet actions()}<Button icon="refresh" onclick={onretry}>Retry</Button>{/snippet}
      </StateCard>
    {/if}
  {:else if row.data.length === 0}
    <StateCard {icon} title={emptyTitle}>{emptyText}</StateCard>
  {:else if visible.length === 0}
    <StateCard icon="search" title="Nothing here matches “{app.search}”">Try a shorter search, or clear it to see everything.</StateCard>
  {:else}
    <div class="grid" class:wrap bind:this={scroller}>
      {#each visible as game (game.universeId)}
        <GameTile {game} onplay={play} onopen={(g) => goto(`/games/${g.universeId}`)} disabled={!canPlay} phase={phaseOf(game)} />
      {/each}
    </div>
  {/if}
</section>

<style>
  section {
    display: grid;
    gap: var(--s-3);
    min-width: 0;
    /* Rows scrolled out of view aren't laid out or painted; Luduvo's home has hundreds of tiles. */
    content-visibility: auto;
    contain-intrinsic-size: auto 230px;
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    color: var(--frost);
  }
  h2 {
    color: var(--text-0);
    font-size: var(--fs-md);
  }
  .count {
    font-size: var(--fs-xs);
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--glass-hover);
    color: var(--text-1);
  }
  .arrows {
    display: flex;
    gap: 4px;
    margin-left: auto;
    margin-right: var(--s-4);
  }
  .arrows button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: var(--glass);
    color: var(--text-1);
    cursor: pointer;
  }
  .arrows button:hover {
    background: var(--glass-hover);
    color: var(--text-0);
    border-color: var(--line-strong);
  }
  /* One line that scrolls sideways; it bleeds to the panel edge. */
  .grid {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 132px;
    gap: var(--s-3);
    overflow-x: auto;
    padding: 6px var(--s-4) var(--s-2) 4px;
    margin: -6px 0 0 -4px;
    scroll-snap-type: x proximity;
    scrollbar-width: none;
  }
  .grid.wrap {
    grid-auto-flow: row;
    grid-template-columns: repeat(auto-fill, minmax(132px, 1fr));
    gap: var(--s-4) var(--s-3);
    overflow: visible;
  }
  .hidden {
    display: none;
  }
  .grid::-webkit-scrollbar {
    display: none;
  }
  .grid > :global(*) {
    scroll-snap-align: start;
  }
  section > :global(.state) {
    margin-right: var(--s-4);
  }
  .ghost {
    display: grid;
    gap: var(--s-2);
  }
  .ghost :global(.skeleton:first-child) {
    aspect-ratio: 1;
  }
</style>
