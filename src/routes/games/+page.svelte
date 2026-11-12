<script lang="ts">
  import { page } from "$app/state";
  import { api } from "$lib/api";
  import GameRow from "$lib/components/GameRow.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { app } from "$lib/state.svelte";
  import { toAppError, type Chart, type Game, type Loadable } from "$lib/types";

  let query = $state(page.url.searchParams.get("q") ?? "");
  let results = $state<Loadable<Game[]>>({ status: "ready", data: [] });
  let charts = $state<Loadable<Chart[]>>({ status: "loading" });
  let latest = 0;

  async function loadCharts() {
    charts = { status: "loading" };
    try {
      charts = { status: "ready", data: await api.gameCharts() };
    } catch (e) {
      charts = { status: "error", error: toAppError(e) };
    }
  }

  async function search(text: string) {
    const ticket = ++latest;
    if (!text.trim()) {
      results = { status: "ready", data: [] };
      return;
    }
    results = { status: "loading" };
    try {
      const data = await api.searchGames(text);
      // Ignore answers to searches the user has already typed past.
      if (ticket === latest) results = { status: "ready", data };
    } catch (e) {
      if (ticket === latest) results = { status: "error", error: toAppError(e) };
    }
  }

  // A search from the title strip arrives as ?q=.
  $effect(() => {
    const q = page.url.searchParams.get("q");
    if (q != null) query = q;
  });

  // Wait for a pause in typing so each keystroke isn't a request to Roblox.
  $effect(() => {
    const text = query;
    const timer = setTimeout(() => void search(text), 350);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    void loadCharts();
  });

  const searching = $derived(query.trim() !== "");
  const asRow = (c: Chart): Loadable<Game[]> => ({ status: "ready", data: c.games });
</script>

<div class="games">
  <header>
    <h1>Games</h1>
    <label class="search">
      <Icon name="search" size={20} />
      <span class="sr-only">Search all of Roblox</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={query} type="search" placeholder="Search all of Roblox" spellcheck="false" autocomplete="off" maxlength="100" autofocus />
      {#if query}<button onclick={() => (query = "")} aria-label="Clear search"><Icon name="close" size={16} /></button>{/if}
    </label>
  </header>

  {#if !app.account}
    <StateCard icon="lock" title="Browsing works signed out">Sign in to press Play. Vodka starts games as the account you pick.</StateCard>
  {/if}

  {#if searching}
    <GameRow
      title="Results for “{query.trim()}”"
      icon="search"
      row={results}
      filter={false}
      wrap
      onretry={() => search(query)}
      emptyTitle="No games found"
      emptyText="Roblox has nothing matching that. Check the spelling or try fewer words."
    />
  {:else if charts.status === "ready" && charts.data.length}
    {#each charts.data as chart (chart.name)}
      <GameRow title={chart.name} icon="games" row={asRow(chart)} filter={false} emptyTitle="" emptyText="" />
    {/each}
  {:else}
    <GameRow
      title="Charts"
      icon="games"
      row={charts.status === "ready" ? { status: "ready", data: [] } : charts}
      filter={false}
      onretry={loadCharts}
      emptyTitle="Roblox's charts are empty right now"
      emptyText="Search for a game above instead."
    />
  {/if}
</div>

<style>
  .games {
    display: grid;
    gap: var(--s-5);
    align-content: start;
  }
  header {
    display: grid;
    gap: var(--s-3);
    margin-right: var(--s-4);
  }
  .games > :global(.state) {
    margin-right: var(--s-4);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    height: 48px;
    padding: 0 var(--s-3) 0 var(--s-4);
    border-radius: var(--r-md);
    background: var(--glass);
    border: 1px solid var(--line-strong);
    color: var(--text-2);
    cursor: text;
    transition:
      border-color var(--fast) var(--ease),
      box-shadow var(--slow) var(--ease);
  }
  .search:focus-within {
    border-color: rgba(164, 222, 255, 0.55);
    box-shadow: 0 0 0 3px rgba(98, 198, 255, 0.14);
    color: var(--frost);
  }
  input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text-0);
    font-size: var(--fs-md);
    outline: none;
    user-select: text;
  }
  input::placeholder {
    color: var(--text-2);
  }
  input::-webkit-search-cancel-button {
    display: none;
  }
  .search button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .search button:hover {
    background: var(--glass-hover);
    color: var(--text-0);
  }
</style>
