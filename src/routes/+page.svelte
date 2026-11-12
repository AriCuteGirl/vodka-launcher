<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Hero from "$lib/components/Hero.svelte";
  import GameRow from "$lib/components/GameRow.svelte";
  import InstallGuide from "$lib/components/InstallGuide.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import { app, signIn } from "$lib/state.svelte";
</script>

<div class="home">
  {#if app.mocktail && !app.mocktail.installed}
    <section class="setup glass" aria-labelledby="setup-title">
      <h2 id="setup-title">Mocktail isn't installed</h2>
      <InstallGuide />
    </section>
  {/if}

  {#if app.account}
    <Hero />
    <GameRow
      title="Continue playing"
      icon="clock"
      row={app.continuePlaying}
      emptyTitle="Nothing to continue yet"
      emptyText="Games you play will show up here so you can get back in with one click."
    />
    <GameRow
      title="Favorites"
      icon="star"
      row={app.favorites}
      emptyTitle="No favorites yet"
      emptyText="Star a game on roblox.com and it appears here next time you open Vodka."
    />
  {:else}
    <section class="signin">
      <Logo size={64} />
      <h1>Sign in to see your games</h1>
      <p class="muted">Vodka shows what you played last and your favorites, with a Play button on each. Your session stays in the system keyring.</p>
      <Button variant="primary" size="lg" busy={app.signingIn} onclick={signIn}>Sign in with Roblox</Button>
    </section>
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
    gap: var(--s-4);
    padding: var(--s-5);
    margin-right: var(--s-4);
    border-color: rgba(255, 213, 133, 0.3);
  }
  .home > :global(.state) {
    margin-right: var(--s-4);
  }
  .signin {
    display: grid;
    justify-items: center;
    align-content: center;
    gap: var(--s-4);
    min-height: 60vh;
    text-align: center;
  }
  .signin p {
    max-width: 46ch;
  }
</style>
