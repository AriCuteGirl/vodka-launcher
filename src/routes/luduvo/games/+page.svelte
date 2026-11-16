<script lang="ts">
  import { page } from "$app/state";
  import { api } from "$lib/api";
  import Icon from "$lib/components/Icon.svelte";
  import LuduvoRow from "$lib/components/LuduvoRow.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { luduvo, startLuduvo } from "$lib/luduvo.svelte";
  import { toAppError, type Loadable, type LuduvoPlace } from "$lib/types";
  import { onMount } from "svelte";

  onMount(startLuduvo);

  let query = $state(page.url.searchParams.get("q") ?? "");
  let results = $state<Loadable<LuduvoPlace[]>>({ status: "loading" });
  let latest = 0;

  // An empty search lists Luduvo's games in its own default order.
  async function search(text: string) {
    const ticket = ++latest;
    results = { status: "loading" };
    try {
      const data = await api.luduvoSearch(text);
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

  // Wait for a pause in typing so each keystroke isn't a request to Luduvo.
  $effect(() => {
    const text = query;
    const timer = setTimeout(() => void search(text), 350);
    return () => clearTimeout(timer);
  });

  const searching = $derived(query.trim() !== "");
</script>

<div class="games">
  <header>
    <h1>Luduvo games</h1>
    <label class="search">
      <Icon name="search" size={20} />
      <span class="sr-only">Search all of Luduvo</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={query} type="search" placeholder="Search all of Luduvo" spellcheck="false" autocomplete="off" maxlength="100" autofocus />
      {#if query}<button onclick={() => (query = "")} aria-label="Clear search"><Icon name="close" size={16} /></button>{/if}
    </label>
  </header>

  {#if luduvo.started && !luduvo.user}
    <StateCard icon="lock" title="Browsing works signed out">Sign in to Luduvo (top right) to press Play.</StateCard>
  {/if}

  <LuduvoRow
    title={searching ? `Results for “${query.trim()}”` : "All games"}
    icon={searching ? "search" : "games"}
    row={results}
    filter={false}
    wrap
    onretry={() => search(query)}
    emptyTitle={searching ? "No games found" : "No games listed"}
    emptyText={searching ? "Luduvo has nothing matching that. Check the spelling or try fewer words." : "Luduvo didn't list any games right now."}
  />
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
    border-color: var(--frost);
    box-shadow: 0 0 0 3px var(--frost-glow);
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
