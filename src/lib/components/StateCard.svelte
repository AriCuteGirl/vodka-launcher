<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon, { type IconName } from "./Icon.svelte";

  let {
    icon,
    title,
    tone = "neutral",
    children,
    actions,
  }: { icon: IconName; title: string; tone?: "neutral" | "warn" | "danger"; children?: Snippet; actions?: Snippet } = $props();
</script>

<div class="state glass {tone}" role={tone === "neutral" ? undefined : "alert"}>
  <span class="badge"><Icon name={icon} size={22} /></span>
  <div class="body">
    <h3>{title}</h3>
    {#if children}<p class="muted">{@render children()}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
  .state {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    padding: var(--s-4) var(--s-5);
  }
  .badge {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: var(--r-md);
    background: var(--glass-hover);
    color: var(--frost);
  }
  .warn .badge {
    color: var(--warn);
  }
  .danger .badge {
    color: var(--danger);
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  h3 {
    font-size: var(--fs-md);
    font-weight: 600;
  }
  p {
    font-size: var(--fs-sm);
    margin-top: 2px;
  }
  .actions {
    display: flex;
    gap: var(--s-2);
  }
</style>
