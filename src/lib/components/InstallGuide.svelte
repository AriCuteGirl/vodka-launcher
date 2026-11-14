<script lang="ts">
  import { app, onWindows, refreshMocktail, toast } from "$lib/state.svelte";
  import Button from "./Button.svelte";
  import CopyCommand from "./CopyCommand.svelte";

  let checking = $state(false);

  async function recheck() {
    checking = true;
    await refreshMocktail().finally(() => (checking = false));
    if (!app.mocktail?.installed) toast(onWindows() ? "Still can't find Roblox. Install it, start any game once from roblox.com, then check again." : "Still can't find Mocktail. Run the commands above, then check again.");
  }
</script>

{#if app.mocktail && !app.mocktail.installed}
  <div class="guide">
    <p class="muted">
      {#if onWindows()}
        Vodka starts the official Roblox client, and it isn't installed yet. Download it from this address, then start any game once from
        roblox.com so Roblox finishes setting itself up:
      {:else if app.mocktail.flatpakAvailable}
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
      {#if !onWindows()}<span class="faint">Installed Mocktail from the AUR or an AppImage? Vodka can't see those yet.</span>{/if}
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
