<script lang="ts">
  import { alerts, dismissAlert } from "$lib/alerts.svelte";
  import Avatar from "./Avatar.svelte";
  import GameLogo from "./GameLogo.svelte";
  import Icon from "./Icon.svelte";

  // Friend pop-ups at the top right, under the title strip.
</script>

<div class="alerts" aria-live="polite">
  {#each alerts.list as a (a.key)}
    <div class="alert {a.game}" role="status">
      <Avatar name={a.name} src={a.avatarUrl} size={36} status={a.join ? "playing" : "online"} />
      <div class="what">
        <span class="from"><GameLogo game={a.game} size={11} />{a.game === "roblox" ? "Roblox" : "Luduvo"}</span>
        <p><b>{a.name}</b> {a.text}</p>
        <div class="row">
          {#if a.join}
            <button
              class="join"
              onclick={() => {
                a.join?.();
                dismissAlert(a.key);
              }}><Icon name="play" size={12} filled />Join</button
            >
          {/if}
          <a href={a.profile} onclick={() => dismissAlert(a.key)}>Profile</a>
        </div>
      </div>
      <button class="x" onclick={() => dismissAlert(a.key)} aria-label="Dismiss"><Icon name="close" size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .alerts {
    position: fixed;
    top: calc(var(--topbar-h) + var(--s-2));
    right: var(--s-4);
    z-index: 40;
    display: grid;
    gap: var(--s-2);
    width: 320px;
    pointer-events: none;
  }
  .alert {
    display: flex;
    align-items: flex-start;
    gap: var(--s-3);
    padding: var(--s-3);
    border-radius: var(--r-md);
    border: 1px solid var(--line-strong);
    background: #141b24;
    box-shadow: var(--shadow-2);
    pointer-events: auto;
    animation: slide-in var(--slow) var(--ease);
  }
  .alert.luduvo {
    border-color: rgba(255, 180, 188, 0.35);
  }
  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateX(16px);
    }
  }
  .what {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 4px;
  }
  .from {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 0.6875rem;
    color: var(--text-2);
  }
  p {
    font-size: var(--fs-sm);
    color: var(--text-1);
    overflow-wrap: anywhere;
  }
  p b {
    color: var(--text-0);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--s-3);
  }
  .join {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 12px;
    border: 0;
    border-radius: var(--r-full);
    background: var(--ok);
    color: #04261a;
    font-size: var(--fs-xs);
    font-weight: 700;
    cursor: pointer;
  }
  .row a {
    font-size: var(--fs-xs);
    color: var(--text-1);
  }
  .row a:hover {
    color: var(--text-0);
  }
  .x {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .x:hover {
    background: var(--glass-hover);
    color: var(--text-0);
  }
  @media (prefers-reduced-motion: reduce) {
    .alert {
      animation: none;
    }
  }
</style>
