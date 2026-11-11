<script lang="ts">
  import { elapsed, memory } from "$lib/format";
  import { app, refreshMocktail, stopInstance } from "$lib/state.svelte";
  import Icon from "./Icon.svelte";

  let now = $state(Date.now());
  let checking = $state(false);

  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
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
  {#each app.instances as inst (inst.id)}
    <section class="glass session" class:mine={inst.accountId === app.account?.id}>
      <header>
        <h2>{inst.phase === "playing" ? "Now playing" : "Now starting"}</h2>
        <span class="time"><Icon name="clock" size={14} />{elapsed((now - inst.startedAtMs) / 1000)}</span>
      </header>
      <p class="game">{inst.game.name}</p>
      <p class="who faint">{inst.accountName} · {Math.round(inst.cpuPercent)}% CPU · {memory(inst.memoryBytes)}</p>
      {#if inst.phase === "starting"}
        <pre class="selectable" aria-label="Mocktail output">{inst.log.slice(-12).join("\n") || "Waiting for Mocktail…"}</pre>
      {/if}
      <div class="row">
        <button class="mini" onclick={() => stopInstance(inst.id, true)}><Icon name="refresh" size={14} />Restart</button>
        <button class="mini danger" onclick={() => stopInstance(inst.id)}><Icon name="close" size={14} />Close</button>
      </div>
    </section>
  {/each}

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
  .session.mine {
    border-color: rgba(164, 222, 255, 0.35);
    box-shadow: var(--glow);
  }
  .who {
    font-size: var(--fs-xs);
    margin-bottom: var(--s-3);
    font-variant-numeric: tabular-nums;
  }
  .row {
    display: flex;
    gap: var(--s-2);
  }
  .mini {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--glass);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
  }
  .mini:hover {
    background: var(--glass-hover);
    border-color: var(--line-strong);
  }
  .mini.danger {
    color: var(--danger);
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
  }
  pre {
    margin: 0;
    max-height: 110px;
    margin-bottom: var(--s-3);
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
