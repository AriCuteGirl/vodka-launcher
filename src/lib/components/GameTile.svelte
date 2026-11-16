<script lang="ts">
  import { compact } from "$lib/format";
  import type { Game } from "$lib/types";
  import Icon from "./Icon.svelte";

  let {
    game,
    onplay,
    onopen,
    disabled = false,
    phase = "idle",
  }: {
    game: Game;
    onplay: (game: Game) => void;
    /** Clicking the tile opens the game's page; the Play pill on hover still starts it straight away. */
    onopen?: (game: Game) => void;
    disabled?: boolean;
    phase?: "idle" | "starting" | "playing";
  } = $props();
  const busy = $derived(phase !== "idle");
  let failed = $state(false);
</script>

{#snippet pill()}
  {#if phase === "starting"}<span class="dot"></span>Starting{:else if phase === "playing"}<span class="dot live"></span>Playing{:else}<Icon name="play" size={16} filled />Play{/if}
{/snippet}

<div class="tile" class:busy class:off={disabled && !onopen}>
  <button
    class="main"
    disabled={disabled && !onopen}
    onclick={() => (onopen ? onopen(game) : onplay(game))}
    aria-label={onopen ? `Open ${game.name}` : `Play ${game.name}`}
    title={game.name}
  >
    <span class="art">
      {#if game.iconUrl && !failed}
        <img src={game.iconUrl} alt="" loading="lazy" decoding="async" onerror={() => (failed = true)} />
      {:else}
        <span class="fallback" aria-hidden="true">{game.name.replace(/[^\p{L}\p{N}]/gu, "").slice(0, 1).toUpperCase() || "?"}</span>
      {/if}
      {#if !onopen}<span class="play" aria-hidden="true">{@render pill()}</span>{/if}
    </span>
    <span class="name">{game.name}</span>
    <span class="meta">
      {#if game.playing != null}<i></i>{compact(game.playing)} playing{:else}&nbsp;{/if}
    </span>
  </button>
  {#if onopen}
    <!-- Its own button, laid over the picture, so a quick start doesn't need the game page. -->
    <div class="over">
      <button class="play quick" disabled={disabled || busy} onclick={() => onplay(game)} aria-label="Play {game.name}">{@render pill()}</button>
    </div>
  {/if}
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
  .art {
    position: relative;
    aspect-ratio: 1;
    margin-bottom: 6px;
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--glass);
    border: 1px solid var(--line);
    box-shadow: var(--shadow-1);
    /* Only the lift and the overlay animate: a blurred shadow or an image filter is repainted on the CPU
       every frame in WebKitGTK, which made long rows stutter under the mouse. */
    transition:
      transform var(--slow) var(--ease),
      border-color var(--fast) var(--ease);
  }
  .art::after {
    content: "";
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.22);
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--slow) var(--ease);
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
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
    z-index: 1;
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
  .tile:focus-within .art,
  .main:focus-visible .art,
  .tile.busy .art {
    transform: translateY(-4px);
    border-color: rgba(164, 222, 255, 0.45);
    box-shadow: var(--glow), var(--shadow-2);
  }
  .tile:not(.off):hover .art::after,
  .main:focus-visible .art::after {
    opacity: 1;
  }
  .tile:not(.off):hover .play,
  .main:focus-visible .play,
  .tile.busy .play {
    opacity: 1;
    transform: translate(-50%, 0);
  }
  /* The quick Play button sits over the picture without being inside the tile's own button. */
  .over {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    aspect-ratio: 1;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: var(--s-3);
    pointer-events: none;
    transition: transform var(--slow) var(--ease);
  }
  .tile:hover .over,
  .tile.busy .over {
    transform: translateY(-4px);
  }
  .play.quick {
    position: static;
    border: 0;
    transform: translateY(8px);
    pointer-events: auto;
    cursor: pointer;
  }
  .tile:hover .play.quick,
  .play.quick:focus-visible,
  .tile.busy .play.quick {
    opacity: 1;
    transform: none;
  }
  .play.quick:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .play.quick:disabled {
    cursor: default;
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
