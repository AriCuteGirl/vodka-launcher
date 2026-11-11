<script lang="ts">
  import { app, play } from "$lib/state.svelte";
  import Button from "./Button.svelte";

  const last = $derived(app.continuePlaying.status === "ready" ? (app.continuePlaying.data[0] ?? null) : null);
  const mine = $derived(last && app.playing?.game.universeId === last.universeId ? app.playing : null);
  const hour = new Date().getHours();
  const greeting = hour < 5 ? "Still up" : hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
</script>

<section class="hero" aria-label="Welcome">
  <!-- Vodka's own artwork: shards of ice drifting in cold light. -->
  <svg class="art" viewBox="0 0 600 240" preserveAspectRatio="xMaxYMid slice" aria-hidden="true">
    <defs>
      <linearGradient id="ice" x1="0" y1="0" x2="1" y2="1">
        <stop stop-color="#eaf7ff" stop-opacity=".55" />
        <stop offset="1" stop-color="#62c6ff" stop-opacity=".05" />
      </linearGradient>
      <radialGradient id="halo" cx="72%" cy="40%" r="55%">
        <stop stop-color="#62c6ff" stop-opacity=".38" />
        <stop offset="1" stop-color="#62c6ff" stop-opacity="0" />
      </radialGradient>
    </defs>
    <rect width="600" height="240" fill="url(#halo)" />
    <g fill="url(#ice)" stroke="#d9f1ff" stroke-opacity=".35" stroke-width="1">
      <path class="s1" d="M408 46l58-20 34 52-26 62-60-14z" />
      <path class="s2" d="M500 132l44-26 30 40-18 52-48-8z" />
      <path class="s3" d="M338 150l34-10 18 32-22 30-34-12z" />
      <path class="s2" d="M534 30l26-8 14 24-16 22-24-10z" />
      <path class="s1" d="M290 40l18-6 10 16-10 16-18-6z" />
    </g>
  </svg>

  <div class="copy">
    <p class="eyebrow">{greeting}, {app.account?.displayName}</p>
    {#if last}
      <h1>Jump back into <span>{last.name}</span></h1>
      <div class="cta">
        {#if mine?.phase === "playing"}
          <span class="live"><i></i>Playing now</span>
        {:else}
          <Button variant="primary" size="lg" icon="play" disabled={!app.mocktail?.installed} busy={mine?.phase === "starting"} onclick={() => play(last)}>
            {mine ? "Starting" : "Play now"}
          </Button>
        {/if}
        {#if !app.mocktail?.installed}<span class="faint">Install Mocktail to play</span>{/if}
      </div>
    {:else}
      <h1>Your next round starts here</h1>
      <p class="muted">Pick a game below and press Play. Vodka starts Mocktail and drops you straight in.</p>
    {/if}
  </div>

  {#if last?.iconUrl}
    <img class="cover" src={last.iconUrl} alt="" />
  {/if}
</section>

<style>
  .hero {
    position: relative;
    display: flex;
    align-items: center;
    min-height: 190px;
    margin-right: var(--s-4);
    padding: var(--s-5) var(--s-6);
    border-radius: var(--r-lg);
    border: 1px solid var(--line-strong);
    background: linear-gradient(120deg, #0d141c 20%, #0b1a26 70%, #0e2433);
    box-shadow: var(--shadow-1);
    overflow: hidden;
  }
  .art {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .s1 {
    animation: drift 9s ease-in-out infinite alternate;
  }
  .s2 {
    animation: drift 12s ease-in-out infinite alternate-reverse;
  }
  .s3 {
    animation: drift 7s ease-in-out infinite alternate;
  }
  @keyframes drift {
    to {
      transform: translate(-6px, 8px) rotate(2deg);
    }
  }
  .copy {
    position: relative;
    display: grid;
    gap: var(--s-3);
    max-width: 60%;
  }
  .eyebrow {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--frost);
  }
  h1 {
    font-size: var(--fs-xl);
    line-height: 1.15;
  }
  h1 span {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .cta {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    font-size: var(--fs-sm);
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    height: 46px;
    padding: 0 var(--s-5);
    border-radius: var(--r-md);
    border: 1px solid rgba(116, 230, 180, 0.4);
    background: rgba(116, 230, 180, 0.1);
    color: var(--ok);
    font-weight: 650;
  }
  .live i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
    box-shadow: 0 0 10px var(--ok);
  }
  .cover {
    position: absolute;
    right: 9%;
    top: 50%;
    width: 132px;
    height: 132px;
    border-radius: var(--r-lg);
    border: 1px solid rgba(255, 255, 255, 0.35);
    box-shadow:
      0 24px 50px rgba(0, 0, 0, 0.6),
      0 0 40px var(--frost-glow);
    transform: translateY(-50%) rotate(5deg);
    transition: transform var(--slow) var(--ease);
  }
  .hero:hover .cover {
    transform: translateY(-52%) rotate(2deg) scale(1.03);
  }
</style>
