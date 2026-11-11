<script lang="ts">
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { elapsed, memory } from "$lib/format";
  import { app, MAX_INSTANCES, refreshAccounts, removeAccount, signIn, stopInstance, switchAccount } from "$lib/state.svelte";
  import type { Account } from "$lib/types";
  import { onMount } from "svelte";

  let now = $state(Date.now());
  let removing = $state<Account | null>(null);

  onMount(() => {
    void refreshAccounts();
    const clock = setInterval(() => (now = Date.now()), 1000);
    // Presence changes on Roblox's side; refresh it now and then while this page is open.
    const presence = setInterval(refreshAccounts, 30_000);
    return () => {
      clearInterval(clock);
      clearInterval(presence);
    };
  });

  function presenceText(a: Account): string {
    if (a.inVodkaGame) return "Playing through Vodka";
    switch (a.presence?.kind) {
      case "in_game":
        return a.presence.location ? `In ${a.presence.location}` : "In a game";
      case "studio":
        return "In Roblox Studio";
      case "online":
        return "Online";
      case "offline":
        return "Offline";
      default:
        return "Presence unavailable";
    }
  }
  const dot = (a: Account) => (a.inVodkaGame || a.presence?.kind === "in_game" ? "playing" : a.presence?.kind === "online" || a.presence?.kind === "studio" ? "online" : undefined);
</script>

<div class="players">
  <header class="top">
    <div>
      <h1>Players</h1>
      <p class="muted">Your Roblox accounts on this computer. Each one keeps its own session and game profile.</p>
    </div>
    <Button variant="primary" icon="plus" busy={app.signingIn} onclick={signIn}>Add account</Button>
  </header>

  {#if app.accounts.length === 0}
    <StateCard icon="players" title="No accounts yet">Add a Roblox account to see your games and start playing.</StateCard>
  {:else}
    <ul class="cards">
      {#each app.accounts as a (a.id)}
        <li class="card glass" class:active={a.active}>
          <Avatar name={a.displayName} src={a.avatarUrl} size={52} status={dot(a)} />
          <div class="who">
            <b>{a.displayName}</b>
            <span class="faint">@{a.name}</span>
            <span class="presence" class:live={!!dot(a)}>{presenceText(a)}</span>
          </div>
          {#if a.active}<span class="badge">Active</span>{/if}
          <div class="actions">
            {#if !a.active}<Button size="sm" onclick={() => switchAccount(a.id)}>Switch to</Button>{/if}
            <Button size="sm" variant="ghost" onclick={() => (removing = a)} aria-label="Remove {a.displayName}">Remove</Button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}

  <section>
    <header class="sub">
      <h2>Running games</h2>
      <span class="count">{app.instances.length} of {MAX_INSTANCES}</span>
    </header>
    {#if app.instances.length === 0}
      <StateCard icon="games" title="Nothing is running">
        Press Play on Home. To run a second game, switch to another account and press Play again.
      </StateCard>
    {:else}
      <ul class="instances">
        {#each app.instances as inst (inst.id)}
          <li class="glass">
            {#if inst.game.iconUrl}<img src={inst.game.iconUrl} alt="" />{:else}<span class="noicon"><Icon name="games" /></span>{/if}
            <div class="what">
              <b>{inst.game.name}</b>
              <span class="faint">{inst.accountName} · {inst.phase === "playing" ? "Playing" : "Starting"} · {elapsed((now - inst.startedAtMs) / 1000)}</span>
            </div>
            <dl>
              <div><dt>CPU</dt><dd>{Math.round(inst.cpuPercent)}%</dd></div>
              <div><dt>Memory</dt><dd>{memory(inst.memoryBytes)}</dd></div>
            </dl>
            <Button size="sm" icon="refresh" onclick={() => stopInstance(inst.id, true)}>Restart</Button>
            <Button size="sm" variant="danger" icon="close" onclick={() => stopInstance(inst.id)}>Close</Button>
          </li>
        {/each}
      </ul>
      <p class="faint note">CPU is a share of one core, so a busy game can show more than 100%.</p>
    {/if}
  </section>
</div>

{#if removing}
  {@const target = removing}
  <Modal title="Remove {target.displayName}?" onclose={() => (removing = null)}>
    <p class="muted">
      Vodka signs this account out of Roblox, deletes its session from the keyring, and deletes its game profile (in-game settings saved on this
      computer). The Roblox account itself is not touched.
    </p>
    {#snippet actions()}
      <Button onclick={() => (removing = null)}>Cancel</Button>
      <Button
        variant="danger"
        onclick={() => {
          removing = null;
          void removeAccount(target.id);
        }}>Remove account</Button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .players {
    display: grid;
    gap: var(--s-5);
    padding-right: var(--s-4);
  }
  .top {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--s-4);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--s-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .card {
    position: relative;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--s-3) var(--s-4);
    align-items: center;
    padding: var(--s-4);
    transition:
      border-color var(--fast) var(--ease),
      box-shadow var(--slow) var(--ease);
  }
  .card.active {
    border-color: rgba(164, 222, 255, 0.4);
    box-shadow: var(--glow);
  }
  .who {
    display: grid;
    min-width: 0;
    line-height: 1.3;
  }
  .who span {
    font-size: var(--fs-sm);
  }
  .presence {
    color: var(--text-2);
  }
  .presence.live {
    color: var(--ok);
  }
  .badge {
    position: absolute;
    top: var(--s-3);
    right: var(--s-3);
    font-size: 0.6875rem;
    font-weight: 650;
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--frost);
    color: var(--frost-ink);
  }
  .actions {
    grid-column: 1 / -1;
    display: flex;
    justify-content: flex-end;
    gap: var(--s-2);
  }
  section {
    display: grid;
    gap: var(--s-3);
  }
  .sub {
    display: flex;
    align-items: center;
    gap: var(--s-2);
  }
  .count {
    font-size: var(--fs-xs);
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--glass-hover);
    color: var(--text-1);
  }
  .instances {
    display: grid;
    gap: var(--s-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .instances li {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4) var(--s-3) var(--s-3);
  }
  img,
  .noicon {
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: var(--r-sm);
  }
  .noicon {
    display: grid;
    place-items: center;
    background: var(--glass-hover);
    color: var(--frost);
  }
  .what {
    display: grid;
    flex: 1;
    min-width: 0;
  }
  .what b {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .what span,
  .note {
    font-size: var(--fs-sm);
  }
  dl {
    display: flex;
    gap: var(--s-4);
    margin: 0 var(--s-2);
    font-variant-numeric: tabular-nums;
  }
  dt {
    font-size: 0.6875rem;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  dd {
    margin: 0;
    font-weight: 600;
    min-width: 6ch;
  }
</style>
