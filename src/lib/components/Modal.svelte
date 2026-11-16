<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let {
    title,
    onclose,
    children,
    actions,
    wide = false,
  }: { title: string; onclose: () => void; children: Snippet; actions?: Snippet; /** Room for a game page. */ wide?: boolean } = $props();
  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
  });
</script>

<dialog bind:this={dialog} class:wide {onclose} onclick={(e) => e.target === dialog && dialog.close()} aria-labelledby="modal-title">
  <div class="panel">
    <header>
      <h2 id="modal-title">{title}</h2>
      <button class="x" onclick={() => dialog.close()} aria-label="Close"><Icon name="close" size={18} /></button>
    </header>
    <div class="content">{@render children()}</div>
    {#if actions}<footer>{@render actions()}</footer>{/if}
  </div>
</dialog>

<style>
  dialog {
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    max-width: min(560px, calc(100vw - 48px));
    width: 100%;
  }
  dialog.wide {
    max-width: min(760px, calc(100vw - 48px));
  }
  dialog::backdrop {
    background: rgba(2, 4, 7, 0.7);
    backdrop-filter: blur(6px);
  }
  .panel {
    background: linear-gradient(180deg, #131a22, var(--bg-1));
    border: 1px solid var(--line-strong);
    border-radius: var(--r-xl);
    box-shadow: var(--shadow-2);
    animation: rise var(--slow) var(--ease);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--s-5) var(--s-5) 0;
  }
  .content {
    padding: var(--s-3) var(--s-5) var(--s-5);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--s-2);
    padding: 0 var(--s-5) var(--s-5);
  }
  .x {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: var(--text-1);
    cursor: pointer;
  }
  .x:hover {
    background: var(--glass-hover);
    color: var(--text-0);
  }
</style>
