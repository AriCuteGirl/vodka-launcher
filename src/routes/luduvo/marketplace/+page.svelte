<script lang="ts">
  import { api } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { compact } from "$lib/format";
  import { luduvo, luduvoSignIn, startLuduvo } from "$lib/luduvo.svelte";
  import { toast } from "$lib/state.svelte";
  import { toAppError, type AppError, type LuduvoCategory, type LuduvoItem } from "$lib/types";
  import { onMount } from "svelte";

  // Luduvo's store: browse and search, open an item, like or favorite it, and buy it after confirming the price.
  onMount(startLuduvo);

  let categories = $state<LuduvoCategory[]>([]);
  let query = $state("");
  let category = $state<LuduvoCategory | null>(null);
  let sort = $state("relevance");
  let items = $state<LuduvoItem[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let more = $state(false);
  let error = $state<AppError | null>(null);
  let latest = 0;

  void api.luduvoStoreCategories().then((c) => (categories = c)).catch(() => {});

  async function search(reset: boolean) {
    const ticket = ++latest;
    if (reset) {
      loading = true;
      error = null;
    } else {
      more = true;
    }
    try {
      const page = await api.luduvoStore(query, category?.assetType ?? "", category?.slot ?? "", sort, reset ? 0 : items.length);
      if (ticket !== latest) return;
      items = reset ? page.items : [...items, ...page.items.filter((i) => !items.some((x) => x.id === i.id))];
      total = page.total;
    } catch (e) {
      if (ticket === latest) error = toAppError(e);
    }
    if (ticket === latest) {
      loading = false;
      more = false;
    }
  }

  // Typing waits for a pause; category and sort apply at once.
  $effect(() => {
    void category;
    void sort;
    const text = query;
    const timer = setTimeout(() => void search(true), text ? 350 : 0);
    return () => clearTimeout(timer);
  });

  // The item pop-up.
  let open = $state<LuduvoItem | null>(null);
  let confirming = $state(false);
  let buying = $state(false);
  let favorited = $state(false);
  async function show(item: LuduvoItem) {
    open = item;
    favorited = false;
    confirming = false;
    try {
      const full = await api.luduvoStoreItem(item.id);
      if (open?.id === item.id) open = full;
    } catch {
      // The tile's own facts are enough to show.
    }
  }
  async function rate(value: number) {
    if (!open || !luduvo.user) return;
    const next = open.myVote === value ? 0 : value;
    try {
      open = await api.luduvoRateItem(open.id, next);
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
  }
  async function favorite() {
    if (!open || !luduvo.user) return;
    try {
      await api.luduvoFavoriteItem(open.id, !favorited);
      favorited = !favorited;
      toast(favorited ? `Added ${open.name} to your favorites.` : `Removed ${open.name} from your favorites.`, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
  }
  async function buy() {
    if (!open || open.price == null) return;
    buying = true;
    try {
      const balance = await api.luduvoBuyItem(open.id, open.price);
      if (balance != null) luduvo.coins = balance;
      toast(`You bought ${open.name}${open.price ? ` for ${open.price} C$` : ""}. It's in your inventory.`, "ok", 7000);
      confirming = false;
    } catch (e) {
      toast(toAppError(e).message, "danger", 9000);
    }
    buying = false;
  }
  const short = $derived(open && open.price != null && luduvo.coins != null && open.price > luduvo.coins);
  const priceText = (p: number | null) => (p == null ? "Off sale" : p === 0 ? "Free" : `${p.toLocaleString()} C$`);
</script>

<div class="market">
  <header>
    <h1>Marketplace</h1>
    {#if luduvo.coins != null}<span class="coins"><b>C$</b> {luduvo.coins.toLocaleString()}</span>{/if}
  </header>

  <div class="bar">
    <label class="search">
      <Icon name="search" size={18} />
      <span class="sr-only">Search Luduvo's store</span>
      <input bind:value={query} type="search" placeholder="Search Luduvo's store" spellcheck="false" autocomplete="off" maxlength="100" />
      {#if query}<button onclick={() => (query = "")} aria-label="Clear search"><Icon name="close" size={15} /></button>{/if}
    </label>
    <select bind:value={sort} aria-label="Sort">
      <option value="relevance">Relevance</option>
      <option value="popular">Popular</option>
      <option value="newest">Newest</option>
      <option value="algorithm">Recommended</option>
    </select>
  </div>

  {#if categories.length}
    <div class="chips" role="radiogroup" aria-label="Category">
      <button role="radio" aria-checked={category == null} class:on={category == null} onclick={() => (category = null)}>All</button>
      {#each categories as c (c.assetType + c.slot)}
        <button role="radio" aria-checked={category === c} class:on={category === c} onclick={() => (category = c)}>{c.name}</button>
      {/each}
    </div>
  {/if}

  {#if loading}
    <div class="grid">{#each Array(12) as _, i (i)}<div class="ghost"><Skeleton height="auto" radius="var(--r-lg)" /><Skeleton width="70%" height="12px" /></div>{/each}</div>
  {:else if error}
    <StateCard icon="alert" tone="danger" title="Couldn't load the store">
      {error.message}
      {#snippet actions()}<Button icon="refresh" onclick={() => search(true)}>Retry</Button>{/snippet}
    </StateCard>
  {:else if items.length === 0}
    <StateCard icon="search" title="Nothing found">Try other words or another category.</StateCard>
  {:else}
    <p class="faint count">{total.toLocaleString()} items</p>
    <ul class="grid">
      {#each items as item (item.id)}
        <li>
          <button class="item" onclick={() => show(item)} title={item.name}>
            <span class="pic">
              {#if item.pictureUrl}<img src={item.pictureUrl} alt="" loading="lazy" decoding="async" />{:else}<Icon name="market" size={30} />{/if}
              {#if item.limited}<em class="badge">Limited</em>{/if}
            </span>
            <b>{item.name}</b>
            <span class="price" class:free={item.price === 0} class:off={item.price == null}>{priceText(item.price)}</span>
          </button>
        </li>
      {/each}
    </ul>
    {#if items.length < total}
      <div class="more"><Button busy={more} onclick={() => search(false)}>Show more</Button></div>
    {/if}
  {/if}
</div>

{#if open}
  {@const item = open}
  <Modal title={item.name} onclose={() => (open = null)}>
    <div class="detail">
      <div class="big">{#if item.pictureUrl}<img src={item.pictureUrl} alt="" />{:else}<Icon name="market" size={48} />{/if}</div>
      <div class="facts">
        <p class="by">by {item.creator || "Luduvo"}</p>
        <p class="bigprice" class:free={item.price === 0}>{priceText(item.price)}</p>
        <dl>
          <div><dt>Sold</dt><dd>{compact(item.sales)}</dd></div>
          {#if item.owners != null}<div><dt>Owners</dt><dd>{compact(item.owners)}</dd></div>{/if}
          <div><dt>Type</dt><dd>{item.slot || item.assetType}</dd></div>
        </dl>
        {#if item.expiresAt}<p class="faint">On sale until {new Date(item.expiresAt * 1000).toLocaleDateString()}</p>{/if}
        <div class="row">
          <button class="pill" class:liked={item.myVote === 1} disabled={!luduvo.user} onclick={() => rate(1)}><Icon name="thumb" size={15} filled={item.myVote === 1} />{compact(item.likes)}</button>
          <button class="pill" class:disliked={item.myVote === -1} disabled={!luduvo.user} onclick={() => rate(-1)}><span class="down"><Icon name="thumb" size={15} filled={item.myVote === -1} /></span>{compact(item.dislikes)}</button>
          <button class="pill" class:on={favorited} disabled={!luduvo.user} onclick={favorite}><Icon name="star" size={15} filled={favorited} />{favorited ? "Favorited" : "Favorite"}</button>
        </div>
      </div>
    </div>
    {#if item.description}<p class="desc selectable">{item.description}</p>{/if}
    {#if confirming && item.price != null}
      <div class="confirm" role="alert">
        <b>Buy {item.name} for {priceText(item.price)}?</b>
        {#if luduvo.coins != null}<span>You have {luduvo.coins.toLocaleString()} C${item.price ? `, ${Math.max(0, luduvo.coins - item.price).toLocaleString()} C$ after.` : "."}</span>{/if}
        {#if short}<span class="err">You don't have enough C$ for this.</span>{/if}
      </div>
    {/if}
    {#snippet actions()}
      <Button onclick={() => (open = null)}>Close</Button>
      {#if !luduvo.user}
        <Button variant="primary" busy={luduvo.signingIn} onclick={luduvoSignIn}>Sign in to buy</Button>
      {:else if item.price == null}
        <Button variant="primary" disabled>Off sale</Button>
      {:else if !confirming}
        <Button variant="primary" onclick={() => (confirming = true)}>{item.price === 0 ? "Get it free" : `Buy for ${item.price} C$`}</Button>
      {:else}
        <Button variant="primary" busy={buying} disabled={!!short} onclick={buy}>{item.price === 0 ? "Yes, get it" : `Yes, pay ${item.price} C$`}</Button>
      {/if}
    {/snippet}
  </Modal>
{/if}

<style>
  .market {
    display: grid;
    gap: var(--s-4);
    align-content: start;
    padding-right: var(--s-4);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .coins {
    padding: 4px 12px;
    border-radius: var(--r-full);
    background: var(--glass-strong);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .coins b {
    color: var(--frost);
    font-size: var(--fs-xs);
  }
  .bar {
    display: flex;
    gap: var(--s-2);
  }
  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--s-3);
    height: 44px;
    padding: 0 var(--s-3) 0 var(--s-4);
    border-radius: var(--r-md);
    background: var(--glass);
    border: 1px solid var(--line-strong);
    color: var(--text-2);
  }
  .search:focus-within {
    border-color: var(--frost);
    box-shadow: 0 0 0 3px var(--frost-glow);
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
  .search button {
    border: 0;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  select {
    height: 44px;
    padding: 0 var(--s-3);
    border-radius: var(--r-md);
    border: 1px solid var(--line-strong);
    background: var(--bg-2);
    color: var(--text-0);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
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
  .count {
    font-size: var(--fs-xs);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
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
    gap: 4px;
    width: 100%;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .pic {
    position: relative;
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    border-radius: var(--r-lg);
    border: 1px solid var(--line);
    background: radial-gradient(circle at 50% 40%, var(--glass-strong), var(--glass));
    color: var(--text-2);
    overflow: hidden;
    transition: transform var(--slow) var(--ease), border-color var(--fast) var(--ease);
  }
  .item:hover .pic {
    transform: translateY(-3px);
    border-color: var(--frost);
  }
  .pic img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .badge {
    position: absolute;
    top: 6px;
    left: 6px;
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--warn);
    color: #2b1d00;
    font-size: 0.6875rem;
    font-style: normal;
    font-weight: 700;
  }
  .item b {
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .price {
    font-size: var(--fs-sm);
    font-weight: 700;
    color: var(--frost);
  }
  .price.free {
    color: var(--ok);
  }
  .price.off {
    color: var(--text-2);
    font-weight: 500;
  }
  .more {
    display: flex;
    justify-content: center;
  }
  .detail {
    display: flex;
    gap: var(--s-5);
  }
  .big {
    display: grid;
    place-items: center;
    width: 180px;
    height: 180px;
    flex: none;
    border-radius: var(--r-lg);
    background: radial-gradient(circle at 50% 40%, var(--glass-strong), var(--glass));
    color: var(--text-2);
  }
  .big img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .facts {
    display: grid;
    gap: var(--s-2);
    align-content: start;
  }
  .by {
    color: var(--text-2);
    font-size: var(--fs-sm);
  }
  .bigprice {
    font-size: var(--fs-xl);
    font-weight: 800;
    color: var(--frost);
  }
  .bigprice.free {
    color: var(--ok);
  }
  dl {
    display: flex;
    gap: var(--s-4);
    margin: 0;
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
    text-transform: capitalize;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    padding: 0 12px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    background: var(--glass);
    color: var(--text-0);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
  }
  .pill:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .pill.liked {
    color: var(--ok);
    border-color: rgba(116, 230, 180, 0.45);
  }
  .pill.disliked {
    color: var(--danger);
    border-color: rgba(255, 155, 155, 0.45);
  }
  .pill.on {
    color: var(--frost);
    border-color: var(--frost);
  }
  .down {
    display: inline-flex;
    transform: rotate(180deg);
  }
  .desc {
    margin-top: var(--s-4);
    font-size: var(--fs-sm);
    color: var(--text-1);
    white-space: pre-wrap;
  }
  .confirm {
    display: grid;
    gap: 4px;
    margin-top: var(--s-4);
    padding: var(--s-3) var(--s-4);
    border-radius: var(--r-md);
    border: 1px solid var(--frost);
    background: var(--glass-strong);
    font-size: var(--fs-sm);
  }
  .err {
    color: var(--danger);
  }
</style>
