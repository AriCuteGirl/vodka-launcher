<script lang="ts">
  import "$lib/styles/app.css";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Onboarding from "$lib/components/Onboarding.svelte";
  import RightRail from "$lib/components/RightRail.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Toasts from "$lib/components/Toasts.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import { app, boot, play } from "$lib/state.svelte";
  import { onMount } from "svelte";

  let { children } = $props();
  onMount(boot);
</script>

{#if !app.booted}
  <div class="splash" role="status"><Logo size={72} /><span class="sr-only">Starting Vodka Launcher</span></div>
{:else if app.bootError}
  <div class="splash">
    <Icon name="alert" size={40} />
    <h1>Vodka couldn't start</h1>
    <p class="muted selectable">{app.bootError.message}</p>
    <Button variant="primary" icon="refresh" onclick={() => location.reload()}>Try again</Button>
  </div>
{:else}
  <div class="shell">
    <TopBar />
    <div class="frame">
      <Sidebar />
      <div class="panel">
        {#if !app.online}
          <div class="offline" role="status"><Icon name="wifiOff" size={16} />You're offline. Games and thumbnails will load again when the connection is back.</div>
        {/if}
        <div class="body">
          <main>{@render children()}</main>
          <RightRail />
        </div>
      </div>
    </div>
  </div>
  {#if !app.settings.onboarded}<Onboarding />{/if}
{/if}

{#if app.crash}
  {@const crash = app.crash}
  <Modal title="{crash.game.name} stopped unexpectedly" onclose={() => (app.crash = null)}>
    <p class="muted">{crash.report.detail ?? "Mocktail closed with an error."}</p>
    {#if crash.log.length}
      <pre class="crashlog selectable">{crash.log.join("\n")}</pre>
    {/if}
    <p class="faint hint">Full log: <code class="selectable">~/.var/app/space.bigrat.mocktail/.local/state/mocktail/logs/latest.log</code></p>
    {#snippet actions()}
      <Button onclick={() => (app.crash = null)}>Close</Button>
      <Button
        variant="primary"
        icon="play"
        onclick={() => {
          app.crash = null;
          void play(crash.game);
        }}>Try again</Button
      >
    {/snippet}
  </Modal>
{/if}

<Toasts />

<style>
  .splash {
    position: fixed;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: var(--s-4);
    text-align: center;
    color: var(--danger);
  }
  .splash :global(svg[role="img"]) {
    animation: breathe 1.6s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      opacity: 0.45;
    }
  }
  .splash h1 {
    color: var(--text-0);
    font-size: var(--fs-xl);
  }
  .shell {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .frame {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .panel {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    margin: 0 var(--s-2) var(--s-2) 0;
    border-radius: var(--r-lg);
    border: 1px solid var(--line);
    background: linear-gradient(180deg, rgba(196, 228, 255, 0.035), rgba(196, 228, 255, 0.012)), var(--bg-1);
    box-shadow: var(--shadow-1);
    overflow: hidden;
  }
  .offline {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s-2);
    padding: 7px var(--s-4);
    background: rgba(255, 213, 133, 0.1);
    border-bottom: 1px solid rgba(255, 213, 133, 0.25);
    color: var(--warn);
    font-size: var(--fs-sm);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: var(--s-4);
  }
  main {
    flex: 1;
    min-width: 0;
    padding: var(--s-4) 0 var(--s-6) var(--s-4);
    overflow-y: auto;
  }
  .crashlog {
    margin: var(--s-3) 0;
    max-height: 200px;
    padding: var(--s-3);
    overflow: auto;
    border-radius: var(--r-md);
    background: rgba(0, 0, 0, 0.45);
    border: 1px solid var(--line);
    color: var(--text-1);
    font-size: var(--fs-xs);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .hint {
    font-size: var(--fs-xs);
  }
  .hint code {
    font-size: inherit;
    word-break: break-all;
  }
  @media (max-width: 1080px) {
    .body :global(.rail) {
      display: none;
    }
    main {
      padding-right: var(--s-4);
    }
  }
</style>
