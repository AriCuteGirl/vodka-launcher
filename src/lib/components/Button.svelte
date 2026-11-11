<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Icon, { type IconName } from "./Icon.svelte";

  type Props = HTMLButtonAttributes & {
    variant?: "primary" | "glass" | "ghost" | "danger";
    size?: "sm" | "md" | "lg";
    icon?: IconName;
    busy?: boolean;
    children?: Snippet;
  };

  let { variant = "glass", size = "md", icon, busy = false, children, disabled, ...rest }: Props = $props();
</script>

<button class="btn {variant} {size}" disabled={disabled || busy} aria-busy={busy} {...rest}>
  {#if busy}
    <span class="spinner" aria-hidden="true"></span>
  {:else if icon}
    <Icon name={icon} size={size === "sm" ? 16 : 18} filled={icon === "play"} />
  {/if}
  {#if children}<span>{@render children()}</span>{/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--s-2);
    height: 38px;
    padding: 0 var(--s-4);
    border-radius: var(--r-md);
    border: 1px solid var(--line);
    background: var(--glass);
    font-weight: 560;
    font-size: var(--fs-sm);
    white-space: nowrap;
    cursor: pointer;
    transition:
      background var(--fast) var(--ease),
      border-color var(--fast) var(--ease),
      box-shadow var(--slow) var(--ease),
      transform var(--fast) var(--ease);
  }
  .btn:hover:not(:disabled) {
    background: var(--glass-hover);
    border-color: var(--line-strong);
  }
  .btn:active:not(:disabled) {
    transform: scale(0.97);
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .sm {
    height: 30px;
    padding: 0 var(--s-3);
    border-radius: var(--r-sm);
  }
  .lg {
    height: 46px;
    padding: 0 var(--s-5);
    font-size: var(--fs-md);
  }
  .primary {
    background: linear-gradient(180deg, #d6f0ff, var(--frost) 45%, var(--frost-strong));
    border-color: rgba(255, 255, 255, 0.5);
    color: var(--frost-ink);
    font-weight: 650;
  }
  .primary:hover:not(:disabled) {
    background: linear-gradient(180deg, #eaf7ff, #b9e6ff 45%, #7fd1ff);
    border-color: #fff;
    box-shadow: var(--glow);
  }
  .ghost {
    background: transparent;
    border-color: transparent;
    color: var(--text-1);
  }
  .ghost:hover:not(:disabled) {
    color: var(--text-0);
  }
  .danger {
    color: var(--danger);
    border-color: rgba(255, 155, 155, 0.3);
  }
  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid currentColor;
    border-right-color: transparent;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
