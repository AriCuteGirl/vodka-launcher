<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import CopyCommand from "$lib/components/CopyCommand.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import LuduvoHero from "$lib/components/LuduvoHero.svelte";
  import LuduvoRow from "$lib/components/LuduvoRow.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { cancelLuduvoSignIn, loadDiscover, loadRecent, luduvo, startLuduvo } from "$lib/luduvo.svelte";
  import { onMount } from "svelte";

  onMount(startLuduvo);

  const missing = $derived(luduvo.status != null && !luduvo.status.installed);
  const unsupported = $derived(luduvo.status != null && !luduvo.status.supported);
</script>

<div class="home">
  <LuduvoHero />

  {#if luduvo.signingIn}
    <StateCard icon="lock" title="Finish signing in in your browser">
      Vodka opened luduvo.com in your browser. Sign in there if you aren't already; Luduvo then hands Vodka a session and you can close the tab.
      {#snippet actions()}<Button onclick={cancelLuduvoSignIn}>Cancel</Button>{/snippet}
    </StateCard>
  {/if}

  {#if unsupported}
    <StateCard icon="alert" tone="warn" title="Luduvo can't be started from Vodka on this system yet">
      You can still browse games here. To play, use the Luduvo app or luduvo.com.
    </StateCard>
  {:else if missing && luduvo.status}
    <section class="setup glass" aria-labelledby="ludinstall">
      <h2 id="ludinstall"><Icon name="alert" size={18} /> Luduvo isn't installed</h2>
      <p class="muted">Install it with Luduvo's own script from luduvo.com/download, then come back. Vodka never runs it for you.</p>
      <CopyCommand label="Linux, x86_64" command={luduvo.status.installCommand} />
      <p class="faint hint">
        The first time Luduvo starts it downloads its game content (about 80 MB) before any window appears. That can take a few
        minutes, and clicking it again in the meantime does nothing.
      </p>
    </section>
  {/if}

  {#if luduvo.user}
    <LuduvoRow
      title="Continue playing"
      icon="clock"
      row={luduvo.recent}
      onretry={loadRecent}
      emptyTitle="Nothing to continue yet"
      emptyText="Luduvo games you play show up here so you can get back in with one click."
    />
  {/if}

  {#if luduvo.discover.status === "ready"}
    {#each luduvo.discover.data as shelf (shelf.id)}
      <LuduvoRow
        title={shelf.title}
        subtitle={shelf.subtitle}
        icon={shelf.id === "active" ? "players" : "star"}
        row={{ status: "ready", data: shelf.places }}
        onretry={loadDiscover}
        emptyTitle="Nothing here yet"
        emptyText="Luduvo has no games on this shelf right now."
      />
    {/each}
  {:else}
    <LuduvoRow
      title="Discover"
      icon="star"
      row={luduvo.discover.status === "error" ? luduvo.discover : { status: "loading" }}
      onretry={loadDiscover}
      emptyTitle="Nothing to discover"
      emptyText="Luduvo didn't list any games."
    />
  {/if}
</div>

<style>
  .home {
    display: grid;
    gap: var(--s-5);
    align-content: start;
    min-height: 100%;
  }
  .setup {
    display: grid;
    gap: var(--s-3);
    padding: var(--s-5);
    margin-right: var(--s-4);
    border-color: rgba(255, 213, 133, 0.3);
  }
  .setup h2 {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-size: var(--fs-lg);
    color: var(--warn);
  }
  .hint {
    font-size: var(--fs-xs);
  }
  .home > :global(.state) {
    margin-right: var(--s-4);
  }
</style>
