<script lang="ts">
  import { api } from "$lib/api";
  import AvatarEditor from "$lib/components/AvatarEditor.svelte";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { matches } from "$lib/format";
  import { app, myInstance, signIn, toast } from "$lib/state.svelte";
  import { toAppError, type AvatarView, type Loadable, type Outfit } from "$lib/types";

  let view = $state<Loadable<AvatarView>>({ status: "loading" });
  let picture = $state<string | null>(null);
  let wearing = $state<number | null>(null);
  let lastWorn = $state<number | null>(null);
  let filter = $state("");
  let tab = $state<"outfits" | "wearing" | "items" | "advanced">("outfits");
  const TABS = [
    { key: "outfits", label: "Outfits" },
    { key: "wearing", label: "Wearing" },
    { key: "items", label: "My items" },
    { key: "advanced", label: "Advanced" },
  ] as const;

  /** After the editor changed the look: redraw the picture once Roblox has, and refresh outfits. */
  function changed() {
    for (const delay of [2500, 6000]) {
      setTimeout(async () => {
        try {
          picture = (await api.avatarPicture()) ?? picture;
        } catch {
          // The old picture stays; Refresh is there.
        }
      }, delay);
    }
  }

  async function load() {
    view = { status: "loading" };
    try {
      const data = await api.avatar();
      picture = data.pictureUrl;
      view = { status: "ready", data };
    } catch (e) {
      view = { status: "error", error: toAppError(e) };
    }
  }

  // Reload when the active account changes.
  $effect(() => {
    void app.account?.id;
    lastWorn = null;
    if (app.account) void load();
  });

  async function wear(outfit: Outfit) {
    if (wearing != null) return;
    wearing = outfit.id;
    try {
      const refused = await api.wearOutfit(outfit.id);
      lastWorn = outfit.id;
      const inGame = myInstance() ? " In your running game it shows after you respawn or rejoin." : "";
      if (refused.length) toast(`Wearing ${outfit.name}, but Roblox refused: ${refused.join(", ")}.${inGame}`, "danger");
      else toast(`Wearing ${outfit.name}.${inGame}`, "ok");
      // Roblox redraws the picture a moment after the change.
      for (const delay of [2500, 6000]) {
        setTimeout(async () => {
          try {
            picture = (await api.avatarPicture()) ?? picture;
          } catch {
            // The old picture stays; Refresh is there.
          }
        }, delay);
      }
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    wearing = null;
  }

  const outfits = $derived(view.status === "ready" ? view.data.outfits.filter((o) => matches(o.name, filter)) : []);
</script>

<div class="avatar">
  <header>
    <div>
      <h1>Avatar</h1>
      <p class="muted">Switch outfits, put items on and off, save a look as an outfit, or set accessories by item ID.</p>
    </div>
  </header>

  {#if !app.account}
    <StateCard icon="lock" title="Sign in to see your avatar">
      Outfits belong to a Roblox account, so pick one first.
      {#snippet actions()}<Button variant="primary" busy={app.signingIn} onclick={signIn}>Sign in</Button>{/snippet}
    </StateCard>
  {:else}
    <div class="layout">
      <section class="now glass">
        <h2>{app.account.displayName} right now</h2>
        <div class="frame">
          {#if picture}<img src={picture} alt="Current avatar of {app.account.displayName}" />
          {:else if view.status === "loading"}<Skeleton height="100%" radius="var(--r-lg)" />
          {:else}<Icon name="avatar" size={64} />{/if}
        </div>
        <Button
          icon="refresh"
          onclick={async () => {
            try {
              picture = (await api.avatarPicture()) ?? picture;
            } catch (e) {
              toast(toAppError(e).message, "danger");
            }
          }}>Refresh picture</Button
        >
        {#if myInstance()}
          <p class="faint note">You're in a game. A new outfit shows there after you respawn or rejoin; some games never reload it.</p>
        {/if}
      </section>

      <section class="outfits">
        <div class="tabs" role="tablist" aria-label="Avatar editor">
          {#each TABS as t (t.key)}
            <button role="tab" aria-selected={tab === t.key} onclick={() => ((tab = t.key), t.key === "outfits" && void load())}>{t.label}</button>
          {/each}
        </div>
        {#if tab !== "outfits"}
          <AvatarEditor {tab} onchanged={changed} />
        {:else}
        <div class="bar">
          <h2>Saved outfits {#if view.status === "ready"}<span class="count">{view.data.outfits.length}</span>{/if}</h2>
          {#if view.status === "ready" && view.data.outfits.length > 8}
            <label class="search">
              <Icon name="search" size={15} /><span class="sr-only">Filter outfits</span>
              <input bind:value={filter} type="search" placeholder="Filter outfits" spellcheck="false" autocomplete="off" />
            </label>
          {/if}
        </div>

        {#if view.status === "loading"}
          <ul class="grid" aria-busy="true" aria-label="Loading outfits">
            {#each Array(8) as _, i (i)}<li class="ghost"><Skeleton height="auto" radius="var(--r-md)" /><Skeleton width="70%" height="12px" /></li>{/each}
          </ul>
        {:else if view.status === "error"}
          {#if view.error.kind === "rate_limited"}
            <StateCard icon="clock" tone="warn" title="Roblox asked us to slow down">
              Wait a minute, then retry.
              {#snippet actions()}<Button icon="refresh" onclick={load}>Retry</Button>{/snippet}
            </StateCard>
          {:else if view.error.kind === "session_expired"}
            <StateCard icon="lock" tone="warn" title="Your Roblox session ended">
              Sign in again to see your outfits.
              {#snippet actions()}<Button variant="primary" busy={app.signingIn} onclick={signIn}>Sign in</Button>{/snippet}
            </StateCard>
          {:else}
            <StateCard icon={view.error.kind === "offline" ? "wifiOff" : "alert"} tone="danger" title="Couldn't load your outfits">
              {view.error.message}
              {#snippet actions()}<Button icon="refresh" onclick={load}>Retry</Button>{/snippet}
            </StateCard>
          {/if}
        {:else if view.data.outfits.length === 0}
          <StateCard icon="avatar" title="No saved outfits yet">Save a look as an outfit in Roblox's avatar editor and it appears here.</StateCard>
        {:else if outfits.length === 0}
          <StateCard icon="search" title="No outfit matches “{filter}”">Try part of its name.</StateCard>
        {:else}
          <ul class="grid">
            {#each outfits as o (o.id)}
              <li>
                <button class="outfit" class:worn={lastWorn === o.id} disabled={wearing != null} onclick={() => wear(o)} aria-label="Wear {o.name}">
                  <span class="pic">
                    {#if o.imageUrl}<img src={o.imageUrl} alt="" loading="lazy" />{:else}<Icon name="avatar" size={40} />{/if}
                    <span class="wear">
                      {#if wearing === o.id}Putting on…{:else if lastWorn === o.id}<Icon name="check" size={14} />Wearing{:else}Wear{/if}
                    </span>
                  </span>
                  <span class="name">{o.name}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        {/if}
      </section>
    </div>
  {/if}
</div>

<style>
  .avatar {
    display: grid;
    gap: var(--s-5);
    align-content: start;
    padding-right: var(--s-4);
  }
  .layout {
    display: grid;
    grid-template-columns: 250px 1fr;
    gap: var(--s-5);
    align-items: start;
  }
  .now {
    display: grid;
    gap: var(--s-3);
    padding: var(--s-4);
    position: sticky;
    top: 0;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-size: var(--fs-md);
  }
  .frame {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    border-radius: var(--r-lg);
    background: radial-gradient(circle at 50% 30%, rgba(98, 198, 255, 0.18), transparent 70%), rgba(0, 0, 0, 0.3);
    border: 1px solid var(--line);
    color: var(--text-2);
    overflow: hidden;
  }
  .frame img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .note {
    font-size: var(--fs-xs);
  }
  .outfits {
    display: grid;
    gap: var(--s-3);
    min-width: 0;
  }
  .tabs {
    display: flex;
    gap: 4px;
    padding: 4px;
    width: fit-content;
    border-radius: var(--r-md);
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--line);
  }
  .tabs button {
    padding: 6px 14px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: var(--text-1);
    font-size: var(--fs-sm);
    font-weight: 600;
    cursor: pointer;
  }
  .tabs button:hover {
    color: var(--text-0);
  }
  .tabs button[aria-selected="true"] {
    background: var(--glass-strong);
    color: var(--frost);
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
  }
  .count {
    font-size: var(--fs-xs);
    font-weight: 500;
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--glass-hover);
    color: var(--text-1);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    width: 220px;
    height: 32px;
    padding: 0 var(--s-3);
    border-radius: var(--r-sm);
    background: var(--glass);
    border: 1px solid var(--line);
    color: var(--text-2);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text-0);
    font-size: var(--fs-sm);
    outline: none;
    user-select: text;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
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
  .outfit {
    display: grid;
    gap: 6px;
    width: 100%;
    padding: 0;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .outfit:disabled {
    cursor: progress;
  }
  .pic {
    position: relative;
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    border-radius: var(--r-md);
    background: var(--glass);
    border: 1px solid var(--line);
    color: var(--text-2);
    overflow: hidden;
    transition:
      transform var(--slow) var(--ease),
      border-color var(--fast) var(--ease),
      box-shadow var(--slow) var(--ease);
  }
  .pic img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .outfit:hover:not(:disabled) .pic,
  .outfit:focus-visible .pic,
  .outfit.worn .pic {
    transform: translateY(-3px);
    border-color: rgba(164, 222, 255, 0.5);
    box-shadow: var(--glow);
  }
  .wear {
    position: absolute;
    left: 50%;
    bottom: var(--s-2);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 12px;
    border-radius: var(--r-full);
    background: linear-gradient(180deg, #d6f0ff, var(--frost) 45%, var(--frost-strong));
    color: var(--frost-ink);
    font-size: var(--fs-xs);
    font-weight: 650;
    white-space: nowrap;
    opacity: 0;
    transform: translate(-50%, 6px);
    transition:
      opacity var(--fast) var(--ease),
      transform var(--slow) var(--ease);
  }
  .outfit:hover .wear,
  .outfit:focus-visible .wear,
  .outfit.worn .wear,
  .outfit:disabled .wear {
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
  @media (max-width: 1000px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .now {
      position: static;
      max-width: 260px;
    }
  }
</style>
