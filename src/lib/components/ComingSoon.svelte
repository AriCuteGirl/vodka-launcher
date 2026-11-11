<script lang="ts">
  import Icon, { type IconName } from "./Icon.svelte";

  let { icon, title, phase, points }: { icon: IconName; title: string; phase: number; points: string[] } = $props();
</script>

<div class="wrap">
  <div class="card glass">
    <span class="badge"><Icon name={icon} size={30} /></span>
    <span class="phase">Phase {phase}</span>
    <h1>{title}</h1>
    <p class="muted">This part of Vodka isn't built yet. Here's what it will do:</p>
    <ul>
      {#each points as point (point)}
        <li><Icon name="check" size={16} />{point}</li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .wrap {
    display: grid;
    place-items: center;
    min-height: 100%;
  }
  .card {
    display: grid;
    justify-items: start;
    gap: var(--s-3);
    max-width: 520px;
    padding: var(--s-6);
    border-radius: var(--r-xl);
  }
  .badge {
    display: grid;
    place-items: center;
    width: 60px;
    height: 60px;
    border-radius: var(--r-lg);
    background: var(--glass-strong);
    color: var(--frost);
    box-shadow: var(--glow);
  }
  .phase {
    margin-top: var(--s-2);
    font-size: var(--fs-xs);
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--frost);
  }
  ul {
    display: grid;
    gap: var(--s-2);
    margin: var(--s-2) 0 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    gap: var(--s-2);
    align-items: flex-start;
    color: var(--text-1);
    font-size: var(--fs-sm);
  }
  li :global(svg) {
    color: var(--frost);
    margin-top: 2px;
  }
</style>
