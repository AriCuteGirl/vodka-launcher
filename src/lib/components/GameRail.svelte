<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { GAMES, gameOf, type GameSection } from "$lib/games";
  import { luduvo } from "$lib/luduvo.svelte";
  import GameLogo from "./GameLogo.svelte";

  // The thin column of games left of the page list, like a game library. Switching keeps your place in each game.
  const current = $derived(gameOf(page.url.pathname));
  const lastPage: Record<string, string> = {};
  $effect(() => {
    lastPage[current.id] = page.url.pathname + page.url.search;
  });

  function pick(game: GameSection) {
    if (game.id === current.id) {
      void goto(game.home);
      return;
    }
    void goto(lastPage[game.id] ?? game.home);
  }
</script>

<nav class="games" aria-label="Games">
  {#each GAMES as game (game.id)}
    <button
      class="game {game.id}"
      class:active={game.id === current.id}
      aria-current={game.id === current.id ? "true" : undefined}
      aria-label={game.id === "luduvo" && luduvo.playing ? `${game.name}, playing ${luduvo.playing.title}` : game.name}
      onclick={() => pick(game)}
    >
      {#if game.id === "luduvo" && luduvo.playing}<i class="live" aria-hidden="true"></i>{/if}
      <span class="mark" aria-hidden="true"><GameLogo game={game.id} size={game.id === "roblox" ? 20 : 18} /></span>
      <span class="tip" aria-hidden="true">{game.name}</span>
    </button>
  {/each}
</nav>

<style>
  .games {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-2);
    width: 52px;
    padding: var(--s-2) 0 var(--s-4);
    border-right: 1px solid var(--line);
    margin-right: var(--s-1);
  }
  .game {
    position: relative;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    padding: 0;
    border: 0;
    border-radius: 12px;
    background: none;
    cursor: pointer;
  }
  .mark {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 11px;
    font-size: 0.9375rem;
    font-weight: 800;
    letter-spacing: -0.02em;
    opacity: 0.55;
    filter: saturate(0.6);
    transition:
      opacity var(--fast) var(--ease),
      filter var(--fast) var(--ease),
      transform var(--fast) var(--ease),
      border-radius var(--slow) var(--ease);
  }
  .roblox .mark {
    background: linear-gradient(145deg, #2a2f38, #12151a);
    color: #ffffff;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }
  .luduvo .mark {
    background: linear-gradient(145deg, #ff4f5e, #c4122a);
    color: #fff1f2;
  }
  .game:hover .mark {
    opacity: 0.9;
    filter: none;
    transform: translateY(-1px);
  }
  .game.active .mark {
    opacity: 1;
    filter: none;
    border-radius: 9px;
    box-shadow:
      0 0 0 2px var(--bg-0),
      0 0 0 3.5px var(--frost);
  }
  .game.active::before {
    content: "";
    position: absolute;
    left: -8px;
    width: 3px;
    height: 20px;
    border-radius: 0 3px 3px 0;
    background: var(--frost);
    box-shadow: 0 0 12px var(--frost-strong);
  }
  /* A game of this kind is running. */
  .live {
    position: absolute;
    right: 0;
    bottom: 0;
    z-index: 1;
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background: var(--ok);
    border: 2px solid var(--bg-0);
  }
  .game:focus-visible {
    outline: 2px solid var(--frost);
    outline-offset: 2px;
  }
  .tip {
    position: absolute;
    left: calc(100% + 12px);
    padding: 5px 10px;
    border-radius: var(--r-sm);
    background: #18202a;
    border: 1px solid var(--line-strong);
    color: var(--text-0);
    font-size: var(--fs-xs);
    font-weight: 600;
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transform: translateX(-4px);
    z-index: 20;
    transition:
      opacity var(--fast) var(--ease),
      transform var(--fast) var(--ease);
  }
  .game:hover .tip,
  .game:focus-visible .tip {
    opacity: 1;
    transform: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .mark,
    .tip {
      transition: none;
    }
  }
</style>
