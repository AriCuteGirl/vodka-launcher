<script lang="ts">
  import { api } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { luduvo, luduvoSignIn, startLuduvo } from "$lib/luduvo.svelte";
  import { toast } from "$lib/state.svelte";
  import { toAppError, type Loadable, type LuduvoAvatar, type LuduvoCategory, type LuduvoOutfit, type LuduvoOwned } from "$lib/types";
  import { onMount } from "svelte";

  // Your Luduvo avatar: what you're wearing, what you own (wear it with a click), and saved outfits.
  onMount(startLuduvo);

  let avatar = $state<Loadable<LuduvoAvatar>>({ status: "loading" });
  let outfits = $state<LuduvoOutfit[]>([]);
  let categories = $state<LuduvoCategory[]>([]);
  let tab = $state<LuduvoCategory | null>(null);
  let owned = $state<Loadable<LuduvoOwned[]>>({ status: "loading" });
  let saving = $state(false);
  let outfitName = $state("");
  let pictureFailed = $state(false);
  let naming = $state(false);
  /** The outfit put on last, until something else changes. */
  let wornOutfit = $state<number | null>(null);
  let brokenPictures = $state(new Set<number>());

  /** Luduvo draws a new outfit's picture a few seconds after it's saved; look again until it's there. */
  async function waitForPictures() {
    for (const ms of [1500, 3000, 5000]) {
      await new Promise((r) => setTimeout(r, ms));
      try {
        outfits = await api.luduvoOutfits();
      } catch {
        return;
      }
      if (outfits.every((o) => o.pictureUrl)) return;
    }
  }

  async function loadAvatar() {
    try {
      avatar = { status: "ready", data: await api.luduvoAvatar() };
      pictureFailed = false;
    } catch (e) {
      avatar = { status: "error", error: toAppError(e) };
    }
  }
  async function loadOwned() {
    owned = { status: "loading" };
    try {
      owned = { status: "ready", data: await api.luduvoInventory(tab?.assetType ?? "", tab?.slot ?? "", 0) };
    } catch (e) {
      owned = { status: "error", error: toAppError(e) };
    }
  }

  $effect(() => {
    if (!luduvo.user) return;
    void loadAvatar();
    void api.luduvoOutfits().then((o) => (outfits = o)).catch(() => {});
    void api.luduvoStoreCategories().then((c) => (categories = c)).catch(() => {});
  });
  $effect(() => {
    void tab;
    if (luduvo.user) void loadOwned();
  });

  const wearing = $derived(avatar.status === "ready" ? avatar.data.wearing : []);
  const isBundle = (i: LuduvoOwned) => i.assetType === "avatar_bundle";
  // A body (bundle) is worn whatever its variant; everything else by item and variant.
  const isWorn = (item: LuduvoOwned) => wearing.some((w) => w.id === item.id && (isBundle(item) || w.variant === item.variant));
  const MAX_ACCESSORIES = 20;

  async function apply(next: LuduvoOwned[], done: string) {
    saving = true;
    try {
      avatar = { status: "ready", data: await api.luduvoSetWearing(next) };
      pictureFailed = false;
      wornOutfit = null;
      toast(done, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    saving = false;
  }
  function toggle(item: LuduvoOwned) {
    if (saving) return;
    if (isWorn(item)) {
      // Luduvo always needs a body on; swap it for another one instead.
      if (isBundle(item)) return toast("You always wear a body. Pick another one to swap it.", "info");
      void apply(wearing.filter((w) => !(w.id === item.id && w.variant === item.variant)), `Took off ${item.name}.`);
      return;
    }
    if (isBundle(item)) {
      // A new body replaces every body part.
      void apply([...wearing.filter((w) => !isBundle(w)), item], `Wearing ${item.name}.`);
    } else if (item.assetType === "accessory") {
      if (wearing.filter((w) => w.assetType === "accessory").length >= MAX_ACCESSORIES) return toast(`You can wear up to ${MAX_ACCESSORIES} accessories.`, "danger");
      void apply([...wearing.filter((w) => w.id !== item.id), item], `Wearing ${item.name}.`);
    } else {
      // A shirt, pants or face replaces whatever is in the same slot.
      const slot = item.slots[0] ?? item.slot;
      void apply([...wearing.filter((w) => isBundle(w) || !(w.slots.includes(slot) || w.assetType === item.assetType)), item], `Wearing ${item.name}.`);
    }
  }
  async function wearOutfit(o: LuduvoOutfit) {
    if (saving) return;
    saving = true;
    try {
      avatar = { status: "ready", data: await api.luduvoWearOutfit(o.id) };
      pictureFailed = false;
      wornOutfit = o.id;
      toast(`Wearing ${o.name}.`, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    saving = false;
  }
  async function saveOutfit(e: SubmitEvent) {
    e.preventDefault();
    if (!outfitName.trim()) return;
    try {
      outfits = await api.luduvoSaveOutfit(outfitName.trim());
      toast(`Saved ${outfitName.trim()}.`, "ok");
      outfitName = "";
      naming = false;
      void waitForPictures();
    } catch (err) {
      toast(toAppError(err).message, "danger");
    }
  }
  async function deleteOutfit(o: LuduvoOutfit) {
    try {
      outfits = await api.luduvoDeleteOutfit(o.id);
      toast(`Deleted ${o.name}.`, "ok");
    } catch (err) {
      toast(toAppError(err).message, "danger");
    }
  }
</script>

<div class="avatar">
  <h1>Avatar</h1>

  {#if !luduvo.user}
    {#if luduvo.started}
      <StateCard icon="lock" title="Sign in to Luduvo to change your avatar">
        {#snippet actions()}<Button variant="primary" busy={luduvo.signingIn} onclick={luduvoSignIn}>Sign in to Luduvo</Button>{/snippet}
      </StateCard>
    {/if}
  {:else if avatar.status === "error"}
    <StateCard icon="alert" tone="danger" title="Couldn't load your avatar">
      {avatar.error.message}
      {#snippet actions()}<Button icon="refresh" onclick={loadAvatar}>Retry</Button>{/snippet}
    </StateCard>
  {:else}
    <div class="layout">
      <aside class="me glass">
        <div class="figure" class:busy={saving}>
          {#if avatar.status === "ready" && avatar.data.pictureUrl && !pictureFailed}
            <img src={avatar.data.pictureUrl} alt="Your avatar" onerror={() => (pictureFailed = true)} />
          {:else if avatar.status === "loading"}
            <Skeleton height="100%" radius="var(--r-lg)" />
          {:else}
            <Icon name="avatar" size={64} />
          {/if}
          {#if saving}<span class="saving">Saving…</span>{/if}
        </div>
        <h2>Wearing <span class="count">{wearing.length}</span></h2>
        {#if wearing.length === 0}
          <p class="faint">Nothing on. Pick something from your items.</p>
        {:else}
          <ul class="worn">
            {#each wearing as w (w.id + ":" + w.variant)}
              <li>
                <span class="mini">{#if w.pictureUrl}<img src={w.pictureUrl} alt="" />{:else}<Icon name="market" size={16} />{/if}</span>
                <span class="nm">{w.name}</span>
                {#if isBundle(w)}
                  <span class="kind">Body</span>
                {:else}
                  <button onclick={() => toggle(w)} disabled={saving} aria-label="Take off {w.name}" title="Take off"><Icon name="close" size={14} /></button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}

      </aside>

      <section class="items">
        <div class="outfit-head">
          <h2>Outfits <span class="count">{outfits.length}</span></h2>
          {#if naming}
            <form class="save" onsubmit={saveOutfit}>
              <!-- svelte-ignore a11y_autofocus -->
              <input bind:value={outfitName} placeholder="Name this look" maxlength="50" spellcheck="false" autocomplete="off" aria-label="Outfit name" autofocus />
              <Button type="submit" size="sm" variant="primary" disabled={!outfitName.trim()}>Save</Button>
              <Button size="sm" onclick={() => ((naming = false), (outfitName = ""))}>Cancel</Button>
            </form>
          {:else}
            <Button size="sm" icon="plus" onclick={() => (naming = true)}>Save current look</Button>
          {/if}
        </div>
        {#if outfits.length === 0}
          <p class="faint">No outfits yet. Put a look together, then save it here to switch back to it in one click.</p>
        {:else}
          <ul class="outfits">
            {#each outfits as o (o.id)}
              <li class:on={wornOutfit === o.id}>
                <button class="card" disabled={saving} onclick={() => wearOutfit(o)} title="Wear {o.name}">
                  <span class="shot">
                    {#if o.pictureUrl && !brokenPictures.has(o.id)}
                      <img src={o.pictureUrl} alt="" loading="lazy" onerror={() => (brokenPictures = new Set([...brokenPictures, o.id]))} />
                    {:else}
                      <Icon name="avatar" size={40} />
                    {/if}
                    {#if wornOutfit === o.id}<span class="tick"><Icon name="check" size={14} /></span>{/if}
                  </span>
                  <span class="nm">{o.name}</span>
                </button>
                <button class="del" onclick={() => deleteOutfit(o)} aria-label="Delete {o.name}" title="Delete {o.name}"><Icon name="close" size={13} /></button>
              </li>
            {/each}
          </ul>
        {/if}

        <h2>Your items</h2>
        <div class="chips" role="radiogroup" aria-label="Kind of item">
          <button role="radio" aria-checked={tab == null} class:on={tab == null} onclick={() => (tab = null)}>All</button>
          {#each categories as c (c.assetType + c.slot)}
            <button role="radio" aria-checked={tab === c} class:on={tab === c} onclick={() => (tab = c)}>{c.name}</button>
          {/each}
        </div>
        {#if owned.status === "loading"}
          <div class="grid">{#each Array(10) as _, i (i)}<Skeleton height="150px" radius="var(--r-lg)" />{/each}</div>
        {:else if owned.status === "error"}
          <StateCard icon="alert" tone="danger" title="Couldn't load your items">
            {owned.error.message}
            {#snippet actions()}<Button icon="refresh" onclick={loadOwned}>Retry</Button>{/snippet}
          </StateCard>
        {:else if owned.data.length === 0}
          <StateCard icon="market" title="Nothing here yet">Get items in the <a href="/luduvo/marketplace">Marketplace</a>; they show up here to wear.</StateCard>
        {:else}
          <ul class="grid">
            {#each owned.data as item (item.id + ":" + item.variant)}
              {@const on = isWorn(item)}
              <li>
                <button class="item" class:on disabled={saving} onclick={() => toggle(item)} title={on ? `Take off ${item.name}` : `Wear ${item.name}`}>
                  <span class="pic">
                    {#if item.pictureUrl}<img src={item.pictureUrl} alt="" loading="lazy" />{:else}<Icon name="market" size={26} />{/if}
                    {#if on}<span class="tick"><Icon name="check" size={14} /></span>{/if}
                  </span>
                  <span class="nm">{item.name}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>
  {/if}
</div>

<style>
  .avatar {
    display: grid;
    gap: var(--s-4);
    align-content: start;
    padding-right: var(--s-4);
  }
  .layout {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: var(--s-5);
    align-items: start;
  }
  .me {
    display: grid;
    gap: var(--s-3);
    padding: var(--s-4);
    position: sticky;
    top: 0;
  }
  .figure {
    position: relative;
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    border-radius: var(--r-lg);
    background: radial-gradient(circle at 50% 35%, var(--glass-strong), transparent 70%);
    color: var(--text-2);
    overflow: hidden;
  }
  .figure.busy img {
    opacity: 0.5;
  }
  .figure img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    transition: opacity var(--fast) var(--ease);
  }
  .saving {
    position: absolute;
    bottom: var(--s-2);
    padding: 2px 10px;
    border-radius: var(--r-full);
    background: rgba(0, 0, 0, 0.6);
    font-size: var(--fs-xs);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-size: var(--fs-sm);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-1);
  }
  .count {
    font-size: var(--fs-xs);
    padding: 0 7px;
    border-radius: var(--r-full);
    background: var(--glass-hover);
    color: var(--text-2);
    letter-spacing: 0;
  }
  .worn {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .worn li {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: 4px 6px;
    border-radius: var(--r-sm);
    background: var(--glass);
    font-size: var(--fs-sm);
  }
  .mini {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: 6px;
    background: var(--glass-strong);
    overflow: hidden;
  }
  .mini img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    font-size: var(--fs-xs);
    color: var(--text-2);
    padding-right: 4px;
  }
  .worn button,
  .del {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .worn button:hover,
  .del:hover {
    color: var(--danger);
    background: var(--glass-hover);
  }
  .outfit-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    flex-wrap: wrap;
  }
  .outfits {
    display: flex;
    gap: var(--s-3);
    margin: 0;
    padding: 0 0 var(--s-2);
    list-style: none;
    overflow-x: auto;
  }
  .outfits li {
    position: relative;
    flex: none;
    width: 128px;
  }
  .card {
    display: grid;
    gap: 6px;
    width: 100%;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .card:disabled {
    cursor: wait;
  }
  .shot {
    position: relative;
    display: grid;
    place-items: center;
    height: 160px;
    border-radius: var(--r-lg);
    border: 1px solid var(--line);
    background: radial-gradient(circle at 50% 40%, var(--glass-strong), var(--glass));
    color: var(--text-2);
    overflow: hidden;
    transition: border-color var(--fast) var(--ease), transform var(--fast) var(--ease);
  }
  .card:hover .shot {
    border-color: var(--frost);
    transform: translateY(-2px);
  }
  .on .shot {
    border: 2px solid var(--frost);
  }
  .shot img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .card .nm {
    font-size: var(--fs-sm);
    font-weight: 600;
  }
  .outfits .del {
    position: absolute;
    top: 6px;
    left: 6px;
    background: rgba(0, 0, 0, 0.55);
    opacity: 0;
    transition: opacity var(--fast) var(--ease);
  }
  .outfits li:hover .del,
  .outfits .del:focus-visible {
    opacity: 1;
  }
  .save {
    display: flex;
    gap: var(--s-2);
  }
  .save input {
    width: 200px;
    height: 32px;
    padding: 0 var(--s-3);
    border-radius: var(--r-sm);
    border: 1px solid var(--line-strong);
    background: rgba(0, 0, 0, 0.25);
    color: var(--text-0);
    font-size: var(--fs-sm);
    user-select: text;
  }
  .items {
    display: grid;
    gap: var(--s-3);
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
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: var(--s-3);
    margin: 0;
    padding: 0;
    list-style: none;
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
  .item:disabled {
    cursor: wait;
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
  }
  .item:hover .pic {
    border-color: var(--line-strong);
  }
  .item.on .pic {
    border: 2px solid var(--frost);
  }
  .pic img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .tick {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--frost);
    color: var(--frost-ink);
  }
  .item .nm {
    font-size: var(--fs-sm);
  }
  @media (max-width: 900px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .me {
      position: static;
    }
  }
</style>
