<script lang="ts">
  import type { GameId } from "$lib/types";

  // Each game's own mark, drawn inline so nothing loads from outside. Colour comes from `currentColor`.
  let { game, size = 20 }: { game: GameId; size?: number } = $props();
</script>

{#if game === "roblox"}
  <!-- Roblox: the tilted square with a square cut-out. -->
  <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
    <path
      fill="currentColor"
      fill-rule="evenodd"
      d="M13.6 .2 63.8 13.6 50.4 63.8 .2 50.4ZM27.4 24 24 36.6 36.6 40 40 27.4Z"
    />
  </svg>
{:else}
  <!-- Luduvo's mark, from luduvo.com's own icon. -->
  <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
    <path
      fill="currentColor"
      fill-rule="evenodd"
      d="M46.47 4.23C53.8 1.04 57.46-.55 59.72.17a6.1 6.1 0 0 1 4.11 4.11c.72 2.26-.88 5.92-4.06 13.25l-4.53 10.44c-.65 1.48-.97 2.23-1.1 3a6.1 6.1 0 0 0 0 2.06c.13.77.45 1.52 1.1 3l4.53 10.44c3.18 7.33 4.78 10.99 4.06 13.25a6.1 6.1 0 0 1-4.11 4.11c-2.26.72-5.92-.88-13.25-4.06l-10.44-4.53c-1.48-.65-2.23-.97-3-1.1a6.1 6.1 0 0 0-2.06 0c-.77.13-1.52.45-3 1.1l-10.44 4.53c-7.33 3.18-10.99 4.78-13.25 4.06A6.1 6.1 0 0 1 .17 59.72c-.72-2.26.88-5.92 4.06-13.25l4.53-10.44c.65-1.48.97-2.23 1.1-3a6.1 6.1 0 0 0 0-2.06c-.13-.77-.45-1.52-1.1-3L4.23 17.53C1.04 10.2-.55 6.54.17 4.28A6.1 6.1 0 0 1 4.28.17C6.54-.55 10.2 1.04 17.53 4.23l10.44 4.53c1.48.65 2.23.97 3 1.1a6.1 6.1 0 0 0 2.06 0c.77-.13 1.52-.45 3-1.1ZM30.53 20.41c-3.54 0-5.31 0-6.67.69a6.3 6.3 0 0 0-2.76 2.76c-.69 1.36-.69 3.13-.69 6.67v2.94c0 3.54 0 5.31.69 6.67a6.3 6.3 0 0 0 2.76 2.76c1.36.69 3.13.69 6.67.69h2.94c3.54 0 5.31 0 6.67-.69a6.3 6.3 0 0 0 2.76-2.76c.69-1.36.69-3.13.69-6.67v-2.94c0-3.54 0-5.31-.69-6.67a6.3 6.3 0 0 0-2.76-2.76c-1.36-.69-3.13-.69-6.67-.69Z"
    />
  </svg>
{/if}
