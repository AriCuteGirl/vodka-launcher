<script lang="ts">
  import { elapsed } from "$lib/format";
  import { app, refreshMocktail } from "$lib/state.svelte";
  import Icon from "./Icon.svelte";

  let now = $state(Date.now());
  let checking = $state(false);
  let logEl = $state<HTMLElement>();

  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  $effect(() => {
    void app.log.length;
    logEl?.scrollTo({ top: logEl.scrollHeight });
  });

  async function recheck() {
    checking = true;
    await refreshMocktail().finally(() => (checking = false));
  }

  const m = $derived(app.mocktail);
</script>

<aside class="rail" aria-label="Status">
  <section class="glass friends">
    <header><h2>Friends</h2><span class="pill">Phase 4</span></header>
    <div class="soon">
      <Icon name="friends" size={26} />
      <p>Who's online, what they're playing, and one-click join.</p>
    </div>
  </section>
  {#if app.playing}
    <section class="glass session">
      <header>
        <h2>Now starting</h2>
        <span class="time"><Icon name="clock" size={14} />{elapsed((now - app.playing.startedAt) / 1000)}</span>
      </header>
      <p class="game">{app.playing.game.name}</p>
      <pre bind:this={logEl} class="selectable" aria-label="Mocktail output">{app.log.slice(-40).join("\n") || "Waiting for Mocktail…"}</pre>
    </section>
  {/if}

  <section class="glass">
    <header>
      <h2>Mocktail</h2>
      <button class="icon" onclick={recheck} aria-label="Check Mocktail again" class:spin={checking}><Icon name="refresh" size={16} /></button>
    </header>
    {#if !m}
      <p class="faint">Looking for Mocktail…</p>
    {:else if m.installed}
      <p class="status ok"><i></i>{m.running ? "Running" : "Ready"}</p>
      <dl>
        <dt>Mocktail</dt>
        <dd>{m.version ?? "unknown"} · Flatpak</dd>
        <dt>Roblox client</dt>
        <dd>{m.robloxVersion ?? "downloads on first launch"}</dd>
      </dl>
    {:else}
      <p class="status bad"><i></i>Not installed</p>
      <p class="faint small">Vodka needs Mocktail to run Roblox. Setup steps are on Home.</p>
    {/if}
  </section>

</aside>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    gap: var(--s-4);
    width: var(--rail-w);
    padding: var(--s-4) var(--s-4) var(--s-4) 0;
    overflow-y: auto;
  }
  section {
    padding: var(--s-4);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--s-3);
  }
  h2 {
    font-size: var(--fs-sm);
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-1);
  }
  .status {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-weight: 600;
  }
  .status i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
    box-shadow: 0 0 10px var(--ok);
  }
  .status.bad i {
    background: var(--warn);
    box-shadow: 0 0 10px var(--warn);
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px var(--s-3);
    margin: var(--s-3) 0 0;
    font-size: var(--fs-sm);
  }
  dt {
    color: var(--text-2);
  }
  dd {
    margin: 0;
    text-align: right;
    color: var(--text-1);
  }
  .small {
    font-size: var(--fs-sm);
    margin-top: var(--s-2);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .icon:hover {
    color: var(--text-0);
    background: var(--glass-hover);
  }
  .spin :global(svg) {
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .session {
    border-color: rgba(164, 222, 255, 0.35);
    box-shadow: var(--glow);
  }
  .time {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-xs);
    color: var(--frost);
    font-variant-numeric: tabular-nums;
  }
  .game {
    font-weight: 600;
    margin-bottom: var(--s-3);
  }
  pre {
    margin: 0;
    max-height: 150px;
    padding: var(--s-2) var(--s-3);
    overflow: auto;
    border-radius: var(--r-sm);
    background: rgba(0, 0, 0, 0.4);
    color: var(--text-1);
    font-size: 0.6875rem;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .friends {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 220px;
  }
  .soon {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: var(--s-2);
    text-align: center;
    color: var(--text-2);
    font-size: var(--fs-sm);
  }
  .soon p {
    max-width: 22ch;
    color: var(--text-1);
  }
  .pill {
    font-size: 0.6875rem;
    padding: 1px 8px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    color: var(--text-2);
  }
</style>
