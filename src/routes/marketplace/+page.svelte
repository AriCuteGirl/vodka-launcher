<script lang="ts">
  import { api } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { app, signIn, toast } from "$lib/state.svelte";
  import { toAppError, type Item, type Loadable, type Offer } from "$lib/types";

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

  // Buying: what the open item costs this account, and the two-step confirmation.
  let offer = $state<Loadable<Offer> | null>(null);
  let confirming = $state(false);
  let buying = $state(false);
  let justBought = $state(false);

  async function show(item: Item) {
    open = item;
    confirming = false;
    justBought = false;
    offer = null;
    if (item.kind === "Bundle" || !app.account) return;
    offer = { status: "loading" };
    try {
      const data = await api.itemOffer(item.id);
      if (open?.id === item.id) offer = { status: "ready", data };
    } catch (e) {
      if (open?.id === item.id) offer = { status: "error", error: toAppError(e) };
    }
  }

  /** Spends Robux. Only reachable from the confirm step, with the price the user is looking at. */
  async function buy(item: Item, price: number) {
    if (buying) return;
    buying = true;
    try {
      const after = await api.buyItem(item.id, price);
      offer = { status: "ready", data: after };
      justBought = true;
      toast(`Bought ${item.name} for ${price.toLocaleString()} Robux.`, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
      // The price or your balance may have changed: show the current state again.
      void show(item);
    }
    confirming = false;
    buying = false;
  }

  async function wearNow(item: Item) {
    try {
      const on = await api.avatarWearing();
      const refused = await api.avatarSetItems([...on.map((i) => i.id), item.id]);
      if (refused.length) toast("Roblox wouldn't put it on. It may be over a limit for its type; try Avatar → Advanced.", "danger");
      else toast(`Wearing ${item.name}.`, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
  }

  const link = (item: Item) => `https://www.roblox.com/${item.kind === "Bundle" ? "bundles" : "catalog"}/${item.id}`;

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
      <p class="muted">Browse and buy from Roblox's avatar catalog. Vodka asks before every purchase and shows the exact price.</p>
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
          <button class="item" onclick={() => show(item)} aria-label="Details for {item.name}">
            <span class="pic">{#if item.imageUrl}<img src={item.imageUrl} alt="" loading="lazy" />{:else}<Icon name="market" size={32} />{/if}</span>
            <span class="name">{item.name}</span>
            {@render cost(item.price, false)}
          </button>
        </li>
      {/each}
    </ul>
    {#if next}<div class="more"><Button busy={loadingMore} onclick={more}>Load more</Button></div>{/if}
  {/if}
</div>

{#snippet cost(amount: number | null, big: boolean)}
  <span class="price" class:big class:free={amount === 0} class:none={amount == null}>
    {#if amount == null}Not for sale{:else if amount === 0}Free{:else}<Icon name="robux" size={big ? 20 : 14} /><span>{amount.toLocaleString()}</span><span class="sr-only">Robux</span>{/if}
  </span>
{/snippet}

{#if open}
  {@const item = open}
  {@const o = offer?.status === "ready" ? offer.data : null}
  <Modal title={item.name} onclose={() => (open = null)}>
    <div class="detail">
      <span class="big-pic">{#if item.imageUrl}<img src={item.imageUrl} alt="" />{:else}<Icon name="market" size={56} />{/if}</span>
      <div class="facts">
        {@render cost(o?.price ?? item.price, true)}
        {#if item.creator}<p class="muted">By <b>{item.creator}</b></p>{/if}
        <p class="faint">{item.kind === "Bundle" ? "Bundle" : "Item"} ID <code class="selectable">{item.id}</code></p>
        {#if o?.owned}<p class="owned"><Icon name="check" size={16} />{justBought ? "It's yours now" : "You own this"}</p>{/if}
        {#if o?.balance != null}<p class="balance faint">Your balance: <Icon name="robux" size={13} />{o.balance.toLocaleString()}</p>{/if}
      </div>
    </div>
    {#if item.description}<p class="desc selectable">{item.description}</p>{/if}

    {#if confirming && o?.price != null}
      <div class="confirm" role="alertdialog" aria-label="Confirm purchase">
        <p><b>Buy {item.name} for <Icon name="robux" size={14} />{o.price.toLocaleString()} Robux?</b></p>
        <p class="muted">
          This spends real Robux from {app.account?.displayName ?? "this account"} and can't be undone here.
          {#if o.balance != null}You'll have <Icon name="robux" size={13} />{(o.balance - o.price).toLocaleString()} left.{/if}
        </p>
        <div class="row">
          <Button onclick={() => (confirming = false)} disabled={buying}>Cancel</Button>
          <Button variant="primary" busy={buying} onclick={() => buy(item, o.price!)}>Yes, buy it</Button>
        </div>
      </div>
    {:else if item.kind === "Bundle"}
      <p class="faint tip">Bundles can't be bought in Vodka yet. Copy the link and buy it on roblox.com.</p>
    {:else if !app.account}
      <p class="faint tip">Sign in to see your balance and buy items.</p>
    {:else if offer?.status === "loading"}
      <Skeleton height="38px" />
    {:else if offer?.status === "error"}
      <p class="err" role="alert">Couldn't check this item: {offer.error.message}</p>
    {:else if o && !o.buyable && !o.owned}
      <p class="faint tip">{o.whyNot}</p>
    {/if}

    {#snippet actions()}
      <Button icon="copy" onclick={() => copy(String(item.id), "ID")}>Copy ID</Button>
      <Button icon="copy" onclick={() => copy(link(item), "Link")}>Copy link</Button>
      {#if !app.account && item.kind !== "Bundle"}
        <Button variant="primary" busy={app.signingIn} onclick={signIn}>Sign in</Button>
      {:else if o?.owned}
        <Button variant="primary" icon="avatar" onclick={() => wearNow(item)}>Wear now</Button>
      {:else if o?.buyable && o.price != null && !confirming}
        <Button variant="primary" onclick={() => (confirming = true)}>{o.price === 0 ? "Get for free" : "Buy"}</Button>
      {/if}
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
  .big-pic {
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
  .big-pic img {
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
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-sm);
    font-weight: 650;
    color: var(--frost);
    font-variant-numeric: tabular-nums;
  }
  .price.none {
    font-weight: 500;
    color: var(--text-2);
  }
  .price.free {
    color: var(--ok);
  }
  .price.big {
    gap: 6px;
    font-size: var(--fs-xl);
    line-height: 1.1;
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
  .facts b {
    color: var(--text-0);
  }
  .owned {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    width: fit-content;
    padding: 3px 10px;
    border-radius: var(--r-full);
    background: rgba(116, 230, 180, 0.12);
    color: var(--ok);
    font-weight: 600;
  }
  .balance {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .desc {
    max-height: 110px;
    margin-top: var(--s-3);
    padding: var(--s-3);
    overflow-y: auto;
    border-radius: var(--r-md);
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--line);
    color: var(--text-1);
    font-size: var(--fs-sm);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .confirm {
    display: grid;
    gap: var(--s-2);
    margin-top: var(--s-3);
    padding: var(--s-4);
    border-radius: var(--r-md);
    border: 1px solid rgba(255, 213, 133, 0.45);
    background: rgba(255, 213, 133, 0.08);
    font-size: var(--fs-sm);
  }
  .confirm p :global(svg) {
    display: inline-block;
    vertical-align: -2px;
    margin-right: 2px;
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: var(--s-2);
    margin-top: var(--s-2);
  }
  .err {
    margin-top: var(--s-3);
    color: var(--danger);
    font-size: var(--fs-sm);
  }
  .tip {
    margin-top: var(--s-3);
    font-size: var(--fs-xs);
  }
</style>
