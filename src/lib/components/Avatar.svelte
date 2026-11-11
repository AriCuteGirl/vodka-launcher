<script lang="ts">
  let { name, src = null, size = 32, status }: { name: string; src?: string | null; size?: number; status?: "online" | "playing" } =
    $props();
  let failed = $state(false);
</script>

<span class="avatar" style:width="{size}px" style:height="{size}px" style:font-size="{size * 0.42}px">
  {#if src && !failed}
    <img {src} alt="" onerror={() => (failed = true)} />
  {:else}
    <span aria-hidden="true">{name.slice(0, 1).toUpperCase()}</span>
  {/if}
  {#if status}<i class={status} title={status === "playing" ? "In a game" : "Online"}></i>{/if}
</span>

<style>
  .avatar {
    position: relative;
    flex: none;
    display: inline-grid;
    place-items: center;
    border-radius: 50%;
    background: linear-gradient(145deg, rgba(164, 222, 255, 0.28), rgba(164, 222, 255, 0.06));
    border: 1px solid var(--line-strong);
    color: var(--frost);
    font-weight: 650;
  }
  img {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
  }
  i {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: 30%;
    height: 30%;
    min-width: 8px;
    min-height: 8px;
    border-radius: 50%;
    border: 2px solid var(--bg-1);
    background: var(--frost-strong);
  }
  i.playing {
    background: var(--ok);
  }
</style>
