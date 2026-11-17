<script lang="ts">
  import { api } from "$lib/api";
  import { hours, lastPlayed, loadPlaytime, playtime } from "$lib/playtime.svelte";
  import type { GameId } from "$lib/types";
  import Icon from "./Icon.svelte";
  import StateCard from "./StateCard.svelte";

  // Every game you've played through Vodka, with how long, Steam-style.
  let { game }: { game: GameId } = $props();

  $effect(() => {
    void loadPlaytime();
  });

  let sort = $state<"recent" | "most">("recent");
  const mine = $derived(playtime.list.filter((p) => p.game === game));
  const rows = $derived(sort === "recent" ? mine : [...mine].sort((a, b) => b.totalSecs - a.totalSecs));
  const total = $derived(mine.reduce((n, p) => n + p.totalSecs, 0));
  const twoWeeks = $derived(mine.reduce((n, p) => n + p.twoWeeksSecs, 0));
  const most = $derived(Math.max(1, ...mine.map((p) => p.totalSecs)));
  const href = (id: number) => (game === "roblox" ? `/games/${id}` : `/luduvo/games/${id}`);

  // Vodka doesn't keep Luduvo pictures with the times; look them up once.
  let icons = $state<Record<number, string | null>>({});
  $effect(() => {
    if (game !== "luduvo") return;
    for (const p of mine) {
      if (p.iconUrl || p.id in icons) continue;
      icons[p.id] = null;
      void api
        .luduvoDetails(p.id)
        .then((d) => (icons[p.id] = d.place.iconUrl ?? d.place.bannerUrl ?? null))
        .catch(() => {});
    }
  });
  let broken = $state(new Set<number>());
</script>

<div class="playtime">
  <h1>Playtime</h1>

  {#if playtime.loaded && mine.length === 0}
    <StateCard icon="clock" title="Nothing counted yet">
      Vodka adds up the time you spend in each game you start from it. Play something and it shows up here.
    </StateCard>
  {:else}
    <div class="totals">
      <div class="glass"><small>On record</small><b>{hours(total)}</b></div>
      <div class="glass"><small>Past 2 weeks</small><b>{twoWeeks ? hours(twoWeeks) : "—"}</b></div>
      <div class="glass"><small>Games</small><b>{mine.length}</b></div>
    </div>

    <div class="chips" role="radiogroup" aria-label="Order">
      <button role="radio" aria-checked={sort === "recent"} class:on={sort === "recent"} onclick={() => (sort = "recent")}>Recently played</button>
      <button role="radio" aria-checked={sort === "most"} class:on={sort === "most"} onclick={() => (sort = "most")}>Most played</button>
    </div>

    <ul class="rows">
      {#each rows as p (p.id)}
        {@const pic = p.iconUrl ?? icons[p.id] ?? null}
        <li>
          <a class="row glass" href={href(p.id)}>
            <span class="icon">
              {#if pic && !broken.has(p.id)}
                <img src={pic} alt="" loading="lazy" onerror={() => (broken = new Set([...broken, p.id]))} />
              {:else}
                <span>{p.title.replace(/[^\p{L}\p{N}]/gu, "").slice(0, 1).toUpperCase() || "?"}</span>
              {/if}
            </span>
            <span class="what">
              <span class="name">{p.title || `Game ${p.id}`}</span>
              <span class="bar" aria-hidden="true"><i style:width="{(p.totalSecs / most) * 100}%"></i></span>
              <span class="meta">Last played {lastPlayed(p.lastPlayed).toLowerCase()}{#if p.twoWeeksSecs > 0} · {hours(p.twoWeeksSecs)} past 2 weeks{/if}</span>
            </span>
            <span class="time"><b>{hours(p.totalSecs)}</b><small>on record</small></span>
            <Icon name="forward" size={16} />
          </a>
        </li>
      {/each}
    </ul>
    <p class="faint note">Counted by Vodka from the moment a game opens until it closes, for games started from Vodka on this computer.</p>
  {/if}
</div>

<style>
  .playtime {
    display: grid;
    gap: var(--s-4);
    align-content: start;
    max-width: 900px;
    padding-right: var(--s-4);
  }
  .totals {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--s-3);
  }
  .totals div {
    display: grid;
    gap: 2px;
    padding: var(--s-3) var(--s-4);
  }
  .totals small {
    font-size: var(--fs-xs);
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .totals b {
    font-size: 1.375rem;
  }
  .chips {
    display: flex;
    gap: 6px;
  }
  .chips button {
    padding: 5px 14px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    background: var(--glass);
    color: var(--text-1);
    font-size: var(--fs-sm);
    font-weight: 600;
    cursor: pointer;
  }
  .chips button.on {
    border-color: var(--frost);
    color: var(--frost);
    background: var(--glass-strong);
  }
  .rows {
    display: grid;
    gap: var(--s-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-2) var(--s-3);
    color: var(--text-2);
    text-decoration: none;
    border: 1px solid transparent;
  }
  .row:hover {
    border-color: var(--line-strong);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    flex: none;
    border-radius: var(--r-md);
    background: var(--glass-strong);
    overflow: hidden;
    font-weight: 700;
    color: var(--text-1);
  }
  .icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .what {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 5px;
  }
  .name {
    color: var(--text-0);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar {
    height: 4px;
    border-radius: var(--r-full);
    background: var(--glass);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--frost);
  }
  .meta {
    font-size: var(--fs-xs);
  }
  .time {
    display: grid;
    justify-items: end;
    min-width: 96px;
  }
  .time b {
    color: var(--text-0);
  }
  .time small {
    font-size: var(--fs-xs);
  }
  .note {
    font-size: var(--fs-xs);
  }
  @media (max-width: 640px) {
    .totals {
      grid-template-columns: 1fr;
    }
  }
</style>
