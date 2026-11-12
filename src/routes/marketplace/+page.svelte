<script lang="ts">
  import { api } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { toast } from "$lib/state.svelte";
  import { toAppError, type Item, type Loadable } from "$lib/types";

  const CATEGORIES = [
    { id: 11, label: "Accessories" },
    { id: 3, label: "Clothing" },
    { id: 4, label: "Body parts" },
    { id: 12, label: "Animations" },
    { id: 1, label: "Everything" },
  ];
  const SORTS = [
    { id: 0, label: "Relevance" },
    { id: 2, label: "Bestselling" },
    { id: 1, label: "Most favorited" },
    { id: 3, label: "Recently updated" },
    { id: 4, label: "Price: low to high" },
    { id: 5, label: "Price: high to low" },
  ];

  let query = $state("");
  let category = $state(11);
  let sort = $state(0);
  let results = $state<Loadable<Item[]>>({ status: "loading" });
  let next = $state<string | null>(null);
  let loadingMore = $state(false);
  let open = $state<Item | null>(null);
  let latest = 0;

  async function search(keyword: string, cat: number, order: number) {
    const ticket = ++latest;
    results = { status: "loading" };
    next = null;
    try {
      const page = await api.catalogSearch(keyword, cat, order);
      // Ignore answers to searches the user has already moved past.
      if (ticket !== latest) return;
      results = { status: "ready", data: page.items };
      next = page.next;
    } catch (e) {
      if (ticket === latest) results = { status: "error", error: toAppError(e) };
    }
  }

  async function more() {
    if (!next || results.status !== "ready") return;
    loadingMore = true;
    try {
      const page = await api.catalogSearch(query, category, sort, next);
      const have = new Set(results.data.map((i) => `${i.kind}:${i.id}`));
      results = { status: "ready", data: [...results.data, ...page.items.filter((i) => !have.has(`${i.kind}:${i.id}`))] };
      next = page.next;
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    loadingMore = false;
  }

  // Wait for a pause in typing so each keystroke isn't a request to Roblox.
  $effect(() => {
    const [q, c, s] = [query, category, sort];
    const timer = setTimeout(() => void search(q, c, s), 400);
    return () => clearTimeout(timer);
  });

  const link = (item: Item) => `https://www.roblox.com/${item.kind === "Bundle" ? "bundles" : "catalog"}/${item.id}`;
  const price = (item: Item) => (item.price == null ? "Not for sale" : item.price === 0 ? "Free" : `${item.price.toLocaleString()} Robux`);

  async function copy(text: string, what: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast(`${what} copied.`, "ok");
    } catch {
      toast("Couldn't reach the clipboard. Select the text and copy it by hand.", "danger");
    }
  }
</script>

<div class="market">
  <header>
    <div>
      <h1>Marketplace</h1>
      <p class="muted">Browse Roblox's avatar catalog. Buying still happens on roblox.com; Vodka never spends Robux.</p>
    </div>
    <div class="filters">
      <label class="search">
        <Icon name="search" size={18} />
        <span class="sr-only">Search the catalog</span>
        <input bind:value={query} type="search" placeholder="Search the catalog" spellcheck="false" autocomplete="off" maxlength="100" />
        {#if query}<button onclick={() => (query = "")} aria-label="Clear search"><Icon name="close" size={15} /></button>{/if}
      </label>
      <label><span class="sr-only">Category</span>
        <select bind:value={category}>{#each CATEGORIES as c (c.id)}<option value={c.id}>{c.label}</option>{/each}</select>
      </label>
      <label><span class="sr-only">Sort by</span>
        <select bind:value={sort}>{#each SORTS as s (s.id)}<option value={s.id}>{s.label}</option>{/each}</select>
      </label>
    </div>
  </header>

  {#if results.status === "loading"}
    <ul class="grid" aria-busy="true" aria-label="Loading items">
      {#each Array(12) as _, i (i)}<li class="ghost"><Skeleton height="auto" radius="var(--r-md)" /><Skeleton width="80%" height="12px" /><Skeleton width="40%" height="10px" /></li>{/each}
    </ul>
  {:else if results.status === "error"}
    {#if results.error.kind === "rate_limited"}
      <StateCard icon="clock" tone="warn" title="Roblox asked us to slow down">
        The catalog allows only a few searches a minute. Wait a moment, then retry.
        {#snippet actions()}<Button icon="refresh" onclick={() => search(query, category, sort)}>Retry</Button>{/snippet}
      </StateCard>
    {:else}
      <StateCard icon={results.error.kind === "offline" ? "wifiOff" : "alert"} tone="danger" title="Couldn't load the catalog">
        {results.error.message}
        {#snippet actions()}<Button icon="refresh" onclick={() => search(query, category, sort)}>Retry</Button>{/snippet}
      </StateCard>
    {/if}
  {:else if results.data.length === 0}
    <StateCard icon="search" title="Nothing found">Try other words, or a different category.</StateCard>
  {:else}
    <ul class="grid">
      {#each results.data as item (`${item.kind}:${item.id}`)}
        <li>
          <button class="item" onclick={() => (open = item)} aria-label="Details for {item.name}">
            <span class="pic">{#if item.imageUrl}<img src={item.imageUrl} alt="" loading="lazy" />{:else}<Icon name="market" size={32} />{/if}</span>
            <span class="name">{item.name}</span>
            <span class="price" class:free={item.price === 0}>{price(item)}</span>
          </button>
        </li>
      {/each}
    </ul>
    {#if next}<div class="more"><Button busy={loadingMore} onclick={more}>Load more</Button></div>{/if}
  {/if}
</div>

{#if open}
  {@const item = open}
  <Modal title={item.name} onclose={() => (open = null)}>
    <div class="detail">
      <span class="big">{#if item.imageUrl}<img src={item.imageUrl} alt="" />{:else}<Icon name="market" size={56} />{/if}</span>
      <div class="facts">
        <p class="price" class:free={item.price === 0}>{price(item)}</p>
        {#if item.creator}<p class="muted">By {item.creator}</p>{/if}
        <p class="faint">{item.kind === "Bundle" ? "Bundle" : "Item"} ID <code class="selectable">{item.id}</code></p>
        {#if item.description}<p class="desc selectable">{item.description}</p>{/if}
      </div>
    </div>
    {#if item.kind !== "Bundle"}
      <p class="faint tip">Own it? Paste the ID into Avatar → Advanced to wear it with up to ten accessories.</p>
    {/if}
    {#snippet actions()}
      <Button icon="copy" onclick={() => copy(String(item.id), "ID")}>Copy ID</Button>
      <Button icon="copy" onclick={() => copy(link(item), "Link")}>Copy link</Button>
      <Button variant="primary" onclick={() => (open = null)}>Done</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  .market {
    display: grid;
    gap: var(--s-5);
    align-content: start;
    padding-right: var(--s-4);
  }
  header {
    display: grid;
    gap: var(--s-3);
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-2);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    flex: 1;
    min-width: 220px;
    height: 42px;
    padding: 0 var(--s-2) 0 var(--s-3);
    border-radius: var(--r-md);
    background: var(--glass);
    border: 1px solid var(--line-strong);
    color: var(--text-2);
    cursor: text;
  }
  .search:focus-within {
    border-color: rgba(164, 222, 255, 0.55);
    box-shadow: 0 0 0 3px rgba(98, 198, 255, 0.14);
    color: var(--frost);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text-0);
    font-size: var(--fs-md);
    outline: none;
    user-select: text;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .search button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
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
  select {
    height: 42px;
    padding: 0 var(--s-3);
    border-radius: var(--r-md);
    border: 1px solid var(--line);
    background: var(--glass);
    color: var(--text-0);
    font-size: var(--fs-sm);
  }
  option {
    background: #141b24;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: var(--s-4) var(--s-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .ghost {
    display: grid;
    gap: var(--s-2);
  }
  .ghost :global(.skeleton:first-child) {
    aspect-ratio: 1;
  }
  .item {
    display: grid;
    gap: 2px;
    width: 100%;
    padding: 0;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
    animation: rise var(--slow) var(--ease) backwards;
  }
  .pic,
  .big {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    border-radius: var(--r-md);
    background: var(--glass);
    border: 1px solid var(--line);
    color: var(--text-2);
    overflow: hidden;
  }
  .pic {
    margin-bottom: 4px;
    transition:
      transform var(--slow) var(--ease),
      border-color var(--fast) var(--ease),
      box-shadow var(--slow) var(--ease);
  }
  .pic img,
  .big img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .item:hover .pic,
  .item:focus-visible .pic {
    transform: translateY(-3px);
    border-color: rgba(164, 222, 255, 0.5);
    box-shadow: var(--glow);
  }
  .name {
    font-size: var(--fs-sm);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .price {
    font-size: var(--fs-xs);
    color: var(--text-1);
    font-variant-numeric: tabular-nums;
  }
  .price.free {
    color: var(--ok);
  }
  .more {
    display: grid;
    justify-items: center;
  }
  .detail {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: var(--s-4);
    align-items: start;
  }
  .facts {
    display: grid;
    gap: 6px;
    font-size: var(--fs-sm);
    min-width: 0;
  }
  .facts .price {
    font-size: var(--fs-lg);
    font-weight: 650;
    color: var(--text-0);
  }
  .facts .price.free {
    color: var(--ok);
  }
  .desc {
    max-height: 130px;
    overflow-y: auto;
    color: var(--text-1);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .tip {
    margin-top: var(--s-3);
    font-size: var(--fs-xs);
  }
</style>
