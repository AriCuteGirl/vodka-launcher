<script lang="ts">
  import { toast } from "$lib/state.svelte";
  import Icon from "./Icon.svelte";

  let { label, command }: { label: string; command: string } = $props();
  let copied = $state(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      copied = true;
      setTimeout(() => (copied = false), 1600);
    } catch {
      toast("Couldn't reach the clipboard. Select the command and copy it by hand.", "danger");
    }
  }
</script>

<div class="cmd">
  <span class="label">{label}</span>
  <div class="row">
    <code class="selectable">{command}</code>
    <button onclick={copy} aria-label="Copy command: {label}">
      <Icon name={copied ? "check" : "copy"} size={16} />
      {copied ? "Copied" : "Copy"}
    </button>
  </div>
</div>

<style>
  .cmd {
    display: grid;
    gap: 6px;
  }
  .label {
    font-size: var(--fs-xs);
    color: var(--text-1);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .row {
    display: flex;
    align-items: stretch;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    overflow: hidden;
  }
  code {
    flex: 1;
    padding: 10px var(--s-3);
    overflow-x: auto;
    white-space: nowrap;
    color: var(--frost);
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 var(--s-3);
    border: 0;
    border-left: 1px solid var(--line);
    background: var(--glass);
    font-size: var(--fs-sm);
    cursor: pointer;
  }
  button:hover {
    background: var(--glass-hover);
  }
  button:focus-visible {
    outline-offset: -2px;
  }
</style>
