<script lang="ts">
  import { app, dismissToast } from "$lib/state.svelte";
  import Icon from "./Icon.svelte";

  // Pop-ups (dialogs) sit in the browser's top layer, above anything else. The toasts go there too, as a popover,
  // and are raised again whenever one appears, so a message from inside a pop-up shows on top of it, not behind.
  let box = $state<HTMLDivElement | null>(null);
  const canPop = typeof HTMLElement !== "undefined" && "showPopover" in HTMLElement.prototype;
  $effect(() => {
    const count = app.toasts.length;
    void app.toasts.map((t) => t.id);
    if (!box || !canPop) return;
    try {
      if (box.matches(":popover-open")) box.hidePopover();
      if (count > 0) box.showPopover();
    } catch {
      // Older engines: the toasts stay a normal layer.
    }
  });
</script>

<div class="toasts" aria-live="polite" bind:this={box} popover={canPop ? "manual" : undefined}>
  {#each app.toasts as t (t.id)}
    <div class="toast {t.tone}" role={t.tone === "danger" ? "alert" : "status"}>
      <Icon name={t.tone === "ok" ? "check" : t.tone === "danger" ? "alert" : "bottle"} size={18} />
      <span>{t.text}</span>
      <button onclick={() => dismissToast(t.id)} aria-label="Dismiss"><Icon name="close" size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    /* The popover defaults (centred, bordered box) are undone so it stays a strip at the bottom. */
    inset: auto;
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    overflow: visible;
    position: fixed;
    left: 50%;
    bottom: var(--s-5);
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    z-index: 50;
    width: max-content;
    max-width: min(520px, calc(100vw - 48px));
  }
  .toast {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-3) var(--s-3) var(--s-4);
    background: #141b24;
    border: 1px solid var(--line-strong);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-2);
    font-size: var(--fs-sm);
    color: var(--frost);
    animation: rise var(--slow) var(--ease);
  }
  .toast span {
    color: var(--text-0);
    flex: 1;
  }
  .ok {
    color: var(--ok);
  }
  .danger {
    color: var(--danger);
    border-color: rgba(255, 155, 155, 0.35);
  }
  button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  button:hover {
    color: var(--text-0);
    background: var(--glass-hover);
  }
</style>
