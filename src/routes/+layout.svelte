<script lang="ts">
  import "$lib/styles/app.css";
  import Button from "$lib/components/Button.svelte";
  import GameRail from "$lib/components/GameRail.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import LaunchDialog from "$lib/components/LaunchDialog.svelte";
  import LuduvoRail from "$lib/components/LuduvoRail.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Onboarding from "$lib/components/Onboarding.svelte";
  import RightRail from "$lib/components/RightRail.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Toasts from "$lib/components/Toasts.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import { api } from "$lib/api";
  import { acceptMultiInstance, app, boot, MAX_INSTANCES, onWindows, play } from "$lib/state.svelte";
  import { page } from "$app/state";
  import { gameOf } from "$lib/games";
  import { onMount } from "svelte";

  let { children } = $props();
  onMount(boot);

  // Errors in the window don't reach the terminal on their own; pass them on so `vodka-launcher` run from a terminal
  // shows what went wrong.
  onMount(() => {
    const onError = (e: ErrorEvent) => void api.uiLog("error", `${e.message} at ${e.filename}:${e.lineno} (page ${location.pathname})`);
    const onReject = (e: PromiseRejectionEvent) => {
      const r = e.reason;
      void api.uiLog("error", `unhandled: ${r instanceof Error ? `${r.message}\n${r.stack ?? ""}` : JSON.stringify(r)} (page ${location.pathname})`);
    };
    window.addEventListener("error", onError);
    window.addEventListener("unhandledrejection", onReject);
    return () => {
      window.removeEventListener("error", onError);
      window.removeEventListener("unhandledrejection", onReject);
    };
  });

  const game = $derived(gameOf(page.url.pathname));
  // The accent colour follows the game; dialogs and toasts live outside the shell, so it goes on the root.
  $effect(() => {
    document.documentElement.dataset.game = game.id;
  });
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
      <GameRail />
      <Sidebar />
      <div class="panel">
        {#if !app.online}
          <div class="offline" role="status"><Icon name="wifiOff" size={16} />You're offline. Games and thumbnails will load again when the connection is back.</div>
        {/if}
        {#if app.batch.length && game.id === "roblox"}
          <div class="batch" role="status">
            <Icon name="players" size={16} />
            <span>
              Launching as <b>{app.batch.length} account{app.batch.length === 1 ? "" : "s"}</b>:
              {app.accounts.filter((a) => app.batch.includes(a.id)).map((a) => a.displayName).join(", ")}. Press Play on a game or Join on a friend.
            </span>
            <button onclick={() => (app.batch = [])}>Clear</button>
          </div>
        {/if}
        <div class="body">
          <main>{@render children()}</main>
          {#if game.id === "roblox"}<RightRail />{:else}<LuduvoRail />{/if}
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
    <p class="faint hint">Played as {crash.accountName}. Mocktail keeps the full log in that account's profile folder.</p>
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

{#if app.pendingLaunch}
  <Modal title="Run several games at once?" onclose={() => (app.pendingLaunch = null)}>
    <div class="warn">
      <p>
        {#if app.pendingLaunch.name}You're about to start <b>{app.pendingLaunch.name}</b> while another game is running.{:else}You're about to let Vodka run several games at once.{/if}
        Before you do:
      </p>
      <ul>
        <li>Roblox's Terms of Use don't allow working around its limit on running clients. Accounts that do can be restricted or banned.</li>
        {#if onWindows()}
          <li>
            On Windows, Vodka does this by holding Roblox's one-client lock before Roblox starts, like other multi-instance tools. Roblox is
            actively blocking such tools, so it may stop working, or get your accounts flagged, after any Roblox update.
          </li>
          <li>Close Roblox before turning this on. Vodka can only allow several games if it starts before the first Roblox window.</li>
        {:else}
          <li>Mocktail's developers chose not to support this. If it breaks, it's on Vodka, not on them.</li>
          <li>Vodka runs one game per account, up to {MAX_INSTANCES}. Each needs about 1.5 GB of memory.</li>
        {/if}
      </ul>
      <p class="faint">This is your call and your risk. Vodka asks once and remembers the answer.</p>
    </div>
    {#snippet actions()}
      <Button onclick={() => (app.pendingLaunch = null)}>Cancel</Button>
      <Button variant="primary" onclick={acceptMultiInstance}>{app.pendingLaunch?.name ? "I understand, start it" : "I understand, turn it on"}</Button>
    {/snippet}
  </Modal>
{/if}

{#if app.launchDialog}<LaunchDialog />{/if}

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
  .batch {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: 8px var(--s-4);
    background: rgba(98, 198, 255, 0.1);
    border-bottom: 1px solid rgba(164, 222, 255, 0.3);
    color: var(--frost);
    font-size: var(--fs-sm);
  }
  .batch span {
    flex: 1;
    min-width: 0;
    color: var(--text-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .batch b {
    color: var(--text-0);
  }
  .batch button {
    padding: 3px 12px;
    border-radius: var(--r-full);
    border: 1px solid var(--line-strong);
    background: var(--glass);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
  }
  .batch button:hover {
    background: var(--glass-hover);
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
  .warn {
    display: grid;
    gap: var(--s-3);
    font-size: var(--fs-sm);
    color: var(--text-1);
  }
  .warn b {
    color: var(--text-0);
  }
  .warn ul {
    display: grid;
    gap: var(--s-2);
    margin: 0;
    padding-left: var(--s-5);
  }
  /* Chat needs the width for its two panes: the rail steps aside sooner there. */
  @media (max-width: 1500px) {
    .body:has(:global(.chat .panes)) :global(.rail) {
      display: none;
    }
  }
  @media (max-width: 1000px) {
    .body :global(.rail) {
      display: none;
    }
    main {
      padding-right: var(--s-4);
    }
  }
</style>
