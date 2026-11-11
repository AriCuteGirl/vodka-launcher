<script lang="ts">
  import { app, refreshMocktail, toast } from "$lib/state.svelte";
  import Button from "./Button.svelte";
  import CopyCommand from "./CopyCommand.svelte";

  let checking = $state(false);

  async function recheck() {
    checking = true;
    await refreshMocktail().finally(() => (checking = false));
    if (!app.mocktail?.installed) toast("Still can't find Mocktail. Run the commands above, then check again.");
  }
</script>

{#if app.mocktail && !app.mocktail.installed}
  <div class="guide">
    <p class="muted">
      {#if app.mocktail.flatpakAvailable}
        Mocktail runs the Roblox client on Linux. Run this in a terminal on {app.mocktail.distro}:
      {:else}
        Vodka finds Mocktail through Flatpak, and Flatpak isn't installed on {app.mocktail.distro} yet. Run these in a terminal, in order:
      {/if}
    </p>
    {#each app.mocktail.installSteps as step (step.label)}
      <CopyCommand label={step.label} command={step.command} />
    {/each}
    <div class="foot">
      <Button variant="primary" icon="refresh" busy={checking} onclick={recheck}>Check again</Button>
      <span class="faint">Installed Mocktail from the AUR or an AppImage? Vodka can't see those yet.</span>
    </div>
  </div>
{/if}

<style>
  .guide {
    display: grid;
    gap: var(--s-4);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    font-size: var(--fs-sm);
  }
</style>
