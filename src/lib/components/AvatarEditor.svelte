<script lang="ts">
  import { api } from "$lib/api";
  import { app, toast } from "$lib/state.svelte";
  import { toAppError, type Item, type Loadable } from "$lib/types";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import Skeleton from "./Skeleton.svelte";
  import StateCard from "./StateCard.svelte";

  /** Which part of the editor to show, and a callback for when the look changed. */
  let { tab, onchanged }: { tab: "wearing" | "items" | "advanced"; onchanged: () => void } = $props();

  const MAX_IDS = 10;
  const KINDS = [
    { key: "accessories", label: "Accessories" },
    { key: "shirts", label: "Shirts" },
    { key: "pants", label: "Pants" },
    { key: "tshirts", label: "T-Shirts" },
    { key: "faces", label: "Faces" },
    { key: "gear", label: "Gear" },
  ];

  let worn = $state<Loadable<Item[]>>({ status: "loading" });
  let kind = $state("accessories");
  let owned = $state<Loadable<Item[]>>({ status: "loading" });
  let next = $state<string | null>(null);
  let loadingMore = $state(false);
  let busy = $state(false);
  let ids = $state<string[]>(Array(MAX_IDS).fill(""));
  let outfitName = $state("");
  let saving = $state(false);

  const wornList = $derived(worn.status === "ready" ? worn.data : []);
  const wornIds = $derived(new Set(wornList.map((i) => i.id)));
  const pretty = (k: string) => k.replace(/([a-z])([A-Z])/g, "$1 $2");

  async function loadWorn() {
    try {
      const data = await api.avatarWearing();
      worn = { status: "ready", data };
      // The Advanced boxes start as the accessories currently on, like Roblox's own dialog.
      const acc = data.filter((i) => i.accessory).map((i) => String(i.id));
      ids = [...acc, ...Array(MAX_IDS).fill("")].slice(0, MAX_IDS);
    } catch (e) {
      worn = { status: "error", error: toAppError(e) };
    }
  }

  async function loadOwned(k: string) {
    owned = { status: "loading" };
    next = null;
    try {
      const page = await api.avatarInventory(k);
      if (kind !== k) return;
      owned = { status: "ready", data: page.items };
      next = page.next;
    } catch (e) {
      if (kind === k) owned = { status: "error", error: toAppError(e) };
    }
  }

  async function more() {
    if (!next || owned.status !== "ready") return;
    loadingMore = true;
    try {
      const page = await api.avatarInventory(kind, next);
      owned = { status: "ready", data: [...owned.data, ...page.items] };
      next = page.next;
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    loadingMore = false;
  }

  $effect(() => {
    void app.account?.id;
    worn = { status: "loading" };
    void loadWorn();
  });
  $effect(() => {
    void app.account?.id;
    if (tab === "items") void loadOwned(kind);
  });

  /** Applies a change, reports anything Roblox refused, and refreshes what's shown. */
  async function apply(run: () => Promise<number[]>, done: string) {
    if (busy) return;
    busy = true;
    try {
      const refused = await run();
      if (refused.length) toast(`Roblox refused ${refused.length === 1 ? "item" : "items"} ${refused.join(", ")}: not owned, not wearable, or over a limit.`, "danger");
      else toast(done, "ok");
      await loadWorn();
      onchanged();
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    busy = false;
  }

  const takeOff = (item: Item) => apply(() => api.avatarSetItems(wornList.filter((i) => i.id !== item.id).map((i) => i.id)), `Took off ${item.name}.`);
  const putOn = (item: Item) => apply(() => api.avatarSetItems([...wornList.map((i) => i.id), item.id]), `Put on ${item.name}.`);

  const entered = $derived(ids.map((v) => v.trim()).filter((v) => v !== ""));
  const badId = $derived(entered.find((v) => !/^\d{1,18}$/.test(v)) ?? null);
  const saveAdvanced = () => apply(() => api.avatarSetAccessories([...new Set(entered.map(Number))]), "Accessories updated.");

  async function saveOutfit() {
    saving = true;
    try {
      await api.avatarSaveOutfit(outfitName);
      toast(`Saved "${outfitName.trim()}" as an outfit. It's on the Outfits tab after a refresh.`, "ok");
      outfitName = "";
      onchanged();
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    saving = false;
  }
</script>

{#snippet card(item: Item, on: boolean, act: () => void, label: string)}
  <li>
    <button class="item" class:on disabled={busy} onclick={act} aria-label="{label} {item.name}" title="{item.name} · {item.id}">
      <span class="pic">
        {#if item.imageUrl}<img src={item.imageUrl} alt="" loading="lazy" />{:else}<Icon name="market" size={30} />{/if}
        <span class="act">{label}</span>
      </span>
      <span class="name">{item.name}</span>
      <span class="kind">{pretty(item.kind)}</span>
    </button>
  </li>
{/snippet}

{#if tab === "wearing"}
  {#if worn.status === "loading"}
    <ul class="grid" aria-busy="true">{#each Array(8) as _, i (i)}<li><Skeleton height="120px" radius="var(--r-md)" /></li>{/each}</ul>
  {:else if worn.status === "error"}
    <StateCard icon="alert" tone="danger" title="Couldn't load what you're wearing">
      {worn.error.message}
      {#snippet actions()}<Button icon="refresh" onclick={loadWorn}>Retry</Button>{/snippet}
    </StateCard>
  {:else if wornList.length === 0}
    <StateCard icon="avatar" title="Nothing on">Put items on from the My items tab.</StateCard>
  {:else}
    <ul class="grid">{#each wornList as item (item.id)}{@render card(item, true, () => takeOff(item), "Take off")}{/each}</ul>
  {/if}
  <form
    class="save glass"
    onsubmit={(e) => {
      e.preventDefault();
      void saveOutfit();
    }}
  >
    <div><b>Save this look as an outfit</b><span class="faint">It appears with your other outfits, here and on Roblox.</span></div>
    <input bind:value={outfitName} placeholder="Outfit name" maxlength="25" aria-label="Outfit name" />
    <Button type="submit" variant="primary" busy={saving} disabled={!outfitName.trim()}>Save outfit</Button>
  </form>
{:else if tab === "items"}
  <div class="kinds" role="tablist" aria-label="Kind of item">
    {#each KINDS as k (k.key)}
      <button role="tab" aria-selected={kind === k.key} onclick={() => ((kind = k.key), void loadOwned(k.key))}>{k.label}</button>
    {/each}
  </div>
  {#if owned.status === "loading"}
    <ul class="grid" aria-busy="true">{#each Array(8) as _, i (i)}<li><Skeleton height="120px" radius="var(--r-md)" /></li>{/each}</ul>
  {:else if owned.status === "error"}
    <StateCard icon="alert" tone="danger" title="Couldn't load your items">
      {owned.error.message}
      {#snippet actions()}<Button icon="refresh" onclick={() => loadOwned(kind)}>Retry</Button>{/snippet}
    </StateCard>
  {:else if owned.data.length === 0}
    <StateCard icon="market" title="You don't own any of these">Items you get on Roblox show up here.</StateCard>
  {:else}
    <ul class="grid">
      {#each owned.data as item (item.id)}
        {@render card(item, wornIds.has(item.id), () => (wornIds.has(item.id) ? takeOff(item) : putOn(item)), wornIds.has(item.id) ? "Take off" : "Wear")}
      {/each}
    </ul>
    {#if next}<div class="more"><Button busy={loadingMore} onclick={more}>Load more</Button></div>{/if}
  {/if}
{:else}
  <form
    class="advanced glass"
    onsubmit={(e) => {
      e.preventDefault();
      void saveAdvanced();
    }}
  >
    <div>
      <b>Advanced: accessories by item ID</b>
      <p class="faint">
        The same box as on roblox.com: up to {MAX_IDS} accessories, including more of one type than the normal editor allows. The boxes start with
        what you have on. Clothes and body parts are left alone. You can copy an item's ID from the Marketplace page.
      </p>
    </div>
    <div class="ids">
      {#each ids as _, i (i)}
        <input bind:value={ids[i]} inputmode="numeric" placeholder="Asset ID" aria-label="Asset ID {i + 1}" class:bad={ids[i].trim() !== "" && !/^\d{1,18}$/.test(ids[i].trim())} />
      {/each}
    </div>
    {#if badId}<p class="err" role="alert">"{badId}" isn't an item ID. IDs are numbers only.</p>{/if}
    <div class="row">
      <Button type="button" variant="ghost" onclick={loadWorn}>Reset to what's on</Button>
      <Button type="submit" variant="primary" {busy} disabled={!!badId}>Save</Button>
    </div>
  </form>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: var(--s-3);
    margin: 0;
    padding: 0;
    list-style: none;
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
  }
  .item:disabled {
    cursor: progress;
  }
  .pic {
    position: relative;
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    margin-bottom: 4px;
    border-radius: var(--r-md);
    background: var(--glass);
    border: 1px solid var(--line);
    color: var(--text-2);
    overflow: hidden;
    transition:
      border-color var(--fast) var(--ease),
      box-shadow var(--slow) var(--ease);
  }
  .pic img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .item.on .pic {
    border-color: rgba(164, 222, 255, 0.55);
    box-shadow: var(--glow);
  }
  .act {
    position: absolute;
    left: 50%;
    bottom: 6px;
    padding: 4px 10px;
    border-radius: var(--r-full);
    background: linear-gradient(180deg, #d6f0ff, var(--frost) 45%, var(--frost-strong));
    color: var(--frost-ink);
    font-size: 0.6875rem;
    font-weight: 650;
    white-space: nowrap;
    opacity: 0;
    transform: translate(-50%, 6px);
    transition:
      opacity var(--fast) var(--ease),
      transform var(--slow) var(--ease);
  }
  .item:hover .act,
  .item:focus-visible .act {
    opacity: 1;
    transform: translate(-50%, 0);
  }
  .name {
    font-size: var(--fs-sm);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .kind {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .kinds {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .kinds button {
    padding: 5px 12px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    background: var(--glass);
    color: var(--text-1);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
  }
  .kinds button[aria-selected="true"] {
    border-color: rgba(164, 222, 255, 0.55);
    background: var(--glass-strong);
    color: var(--frost);
  }
  .more {
    display: grid;
    justify-items: center;
  }
  .save,
  .advanced {
    display: grid;
    gap: var(--s-3);
    padding: var(--s-4);
  }
  .save {
    margin-top: var(--s-4);
    grid-template-columns: 1fr minmax(140px, 220px) auto;
    align-items: center;
  }
  .save div {
    display: grid;
    font-size: var(--fs-sm);
  }
  .save span,
  .advanced p {
    font-size: var(--fs-xs);
  }
  input {
    width: 100%;
    height: 36px;
    padding: 0 var(--s-3);
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: rgba(0, 0, 0, 0.35);
    color: var(--text-0);
    font-size: var(--fs-sm);
    user-select: text;
  }
  input:focus-visible {
    outline: none;
    border-color: rgba(164, 222, 255, 0.5);
    box-shadow: 0 0 0 3px rgba(98, 198, 255, 0.14);
  }
  input.bad {
    border-color: rgba(255, 155, 155, 0.6);
  }
  .ids {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--s-2);
  }
  .ids input {
    font-family: var(--font-mono);
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: var(--s-2);
  }
  .err {
    color: var(--danger);
    font-size: var(--fs-sm);
  }
</style>
