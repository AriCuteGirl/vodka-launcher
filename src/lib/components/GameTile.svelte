<script lang="ts">
  import { compact } from "$lib/format";
  import type { Game } from "$lib/types";
  import Icon from "./Icon.svelte";

  let {
    game,
    onplay,
    onservers,
    disabled = false,
    phase = "idle",
  }: { game: Game; onplay: (game: Game) => void; onservers?: (game: Game) => void; disabled?: boolean; phase?: "idle" | "starting" | "playing" } = $props();
  const busy = $derived(phase !== "idle");
  let failed = $state(false);
</script>

<div class="tile" class:busy class:off={disabled}>
  {#if onservers && !disabled}
    <button class="servers" onclick={() => onservers(game)} aria-label="Choose a server or accounts for {game.name}" title="Servers and accounts">
      <Icon name="settings" size={15} />
    </button>
  {/if}
  <button class="main" {disabled} onclick={() => onplay(game)} aria-label="Play {game.name}" title={game.name}>
  <span class="art">
    {#if game.iconUrl && !failed}
      <img src={game.iconUrl} alt="" loading="lazy" onerror={() => (failed = true)} />
    {:else}
      <span class="fallback" aria-hidden="true">{game.name.replace(/[^\p{L}\p{N}]/gu, "").slice(0, 1).toUpperCase() || "?"}</span>
    {/if}
    <span class="play" aria-hidden="true">
      {#if phase === "starting"}<span class="dot"></span>Starting{:else if phase === "playing"}<span class="dot live"></span>Playing{:else}<Icon name="play" size={16} filled />Play{/if}
    </span>
  </span>
  <span class="name">{game.name}</span>
  <span class="meta">
    {#if game.playing != null}<i></i>{compact(game.playing)} playing{:else}&nbsp;{/if}
  </span>
  </button>
</div>

<style>
  .tile {
    position: relative;
    animation: rise var(--slow) var(--ease) backwards;
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--r-lg);
    text-align: left;
    cursor: pointer;
  }
  .main:disabled {
    cursor: not-allowed;
  }
  .servers {
    position: absolute;
    top: 6px;
    right: 6px;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 9px;
    border: 1px solid var(--line-strong);
    background: rgba(8, 12, 17, 0.82);
    color: var(--text-0);
    cursor: pointer;
    opacity: 0;
    transition:
      opacity var(--fast) var(--ease),
      background var(--fast) var(--ease);
  }
  .tile:hover .servers,
  .servers:focus-visible {
    opacity: 1;
  }
  .servers:hover {
    background: var(--frost);
    color: var(--frost-ink);
  }
  .art {
    position: relative;
    aspect-ratio: 1;
    margin-bottom: 6px;
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--glass);
    border: 1px solid var(--line);
    box-shadow: var(--shadow-1);
    transition:
      transform var(--slow) var(--ease),
      box-shadow var(--slow) var(--ease),
      border-color var(--fast) var(--ease);
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: filter var(--slow) var(--ease);
  }
  .fallback {
    display: grid;
    place-items: center;
    height: 100%;
    font-size: 2.5rem;
    font-weight: 700;
    color: var(--frost);
    background: linear-gradient(150deg, rgba(164, 222, 255, 0.22), transparent 70%);
  }
  .play {
    position: absolute;
    left: 50%;
    bottom: var(--s-3);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px 7px 11px;
    border-radius: var(--r-full);
    background: linear-gradient(180deg, #d6f0ff, var(--frost) 45%, var(--frost-strong));
    color: var(--frost-ink);
    font-size: var(--fs-sm);
    font-weight: 650;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.5);
    opacity: 0;
    transform: translate(-50%, 8px);
    transition:
      opacity var(--fast) var(--ease),
      transform var(--slow) var(--ease);
  }
  .tile:not(.off):hover .art,
  .main:focus-visible .art,
  .tile.busy .art {
    transform: translateY(-4px);
    border-color: rgba(164, 222, 255, 0.45);
    box-shadow: var(--glow), var(--shadow-2);
  }
  .tile:not(.off):hover img,
  .main:focus-visible img {
    filter: brightness(0.8);
  }
  .tile:not(.off):hover .play,
  .main:focus-visible .play,
  .tile.busy .play {
    opacity: 1;
    transform: translate(-50%, 0);
  }
  .tile.off .art {
    filter: grayscale(0.7) brightness(0.7);
  }
  .name {
    font-size: var(--fs-sm);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .meta i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--frost-ink);
    animation: pulse 1s ease-in-out infinite;
  }
  .dot.live {
    background: #0a7a4c;
    animation: none;
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
</style>
