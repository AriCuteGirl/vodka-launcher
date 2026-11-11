<script lang="ts">
  import { app, saveSettings, signIn } from "$lib/state.svelte";
  import Avatar from "./Avatar.svelte";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import InstallGuide from "./InstallGuide.svelte";
  import Logo from "./Logo.svelte";

  let step = $state(0);
  const steps = ["Mocktail", "Account", "Ready"];
  const finish = () => saveSettings({ onboarded: true });
</script>

<div class="onboarding">
  <div class="card">
    <div class="head">
      <Logo size={44} />
      <ol aria-label="Setup progress">
        {#each steps as label, i (label)}
          <li class:done={i < step} class:current={i === step} aria-current={i === step ? "step" : undefined}>
            <span>{#if i < step}<Icon name="check" size={14} />{:else}{i + 1}{/if}</span>{label}
          </li>
        {/each}
      </ol>
    </div>

    {#if step === 0}
      <h1>Welcome to Vodka</h1>
      <p class="lead muted">A launcher for Roblox on Linux. It runs the game through Mocktail, so let's check that first.</p>
      {#if app.mocktail?.installed}
        <div class="found glass">
          <span class="tick"><Icon name="check" size={20} /></span>
          <div>
            <b>Mocktail {app.mocktail.version ?? ""} found</b>
            <p class="faint">Flatpak · Roblox client {app.mocktail.robloxVersion ?? "downloads on first launch"}</p>
          </div>
        </div>
      {:else}
        <InstallGuide />
      {/if}
      <div class="actions">
        <span class="grow"></span>
        {#if !app.mocktail?.installed}<Button variant="ghost" onclick={() => (step = 1)}>Skip for now</Button>{/if}
        <Button variant="primary" size="lg" disabled={!app.mocktail?.installed} onclick={() => (step = 1)}>Continue</Button>
      </div>
    {:else if step === 1}
      <h1>Sign in to Roblox</h1>
      <p class="lead muted">Vodka opens Roblox's own login page. Your password goes to Roblox and nowhere else.</p>
      {#if app.account}
        <div class="found glass">
          <Avatar name={app.account.displayName} src={app.account.avatarUrl} size={44} />
          <div>
            <b>{app.account.displayName}</b>
            <p class="faint">@{app.account.name} · signed in</p>
          </div>
        </div>
      {:else}
        <ul class="facts">
          <li><Icon name="lock" size={18} />Your session is kept in the system keyring, never in a file.</li>
          <li><Icon name="check" size={18} />Vodka only talks to roblox.com. No telemetry, no third parties.</li>
        </ul>
      {/if}
      <div class="actions">
        <Button variant="ghost" onclick={() => (step = 0)}>Back</Button>
        <span class="grow"></span>
        {#if app.account}
          <Button variant="primary" size="lg" onclick={() => (step = 2)}>Continue</Button>
        {:else}
          <Button variant="ghost" onclick={() => (step = 2)}>Skip for now</Button>
          <Button variant="primary" size="lg" busy={app.signingIn} onclick={signIn}>Sign in with Roblox</Button>
        {/if}
      </div>
    {:else}
      <h1>You're set</h1>
      <p class="lead muted">
        {#if app.account && app.mocktail?.installed}
          Pick a game on Home and press Play. Vodka starts Mocktail and drops you straight into it.
        {:else}
          You can look around now. Home will tell you what's still missing before you can play.
        {/if}
      </p>
      <p class="fine faint">Vodka Launcher is an independent project. It is not affiliated with Roblox Corporation, Mocktail, or Lunar Client.</p>
      <div class="actions">
        <Button variant="ghost" onclick={() => (step = 1)}>Back</Button>
        <span class="grow"></span>
        <Button variant="primary" size="lg" icon="play" onclick={finish}>Open Vodka</Button>
      </div>
    {/if}
  </div>
</div>

<style>
  .onboarding {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    padding: var(--s-5);
    background: radial-gradient(800px 500px at 50% 0%, rgba(98, 198, 255, 0.16), transparent 70%), var(--bg-0);
    z-index: 20;
    overflow: auto;
  }
  .card {
    display: grid;
    gap: var(--s-4);
    width: min(600px, 100%);
    padding: var(--s-6);
    border-radius: var(--r-xl);
    background: linear-gradient(180deg, rgba(196, 228, 255, 0.07), rgba(196, 228, 255, 0.02));
    border: 1px solid var(--line-strong);
    box-shadow: var(--shadow-2);
    animation: rise var(--slow) var(--ease);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--s-2);
  }
  ol {
    display: flex;
    gap: var(--s-4);
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-2);
  }
  ol li {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  ol span {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1px solid var(--line-strong);
  }
  ol .current {
    color: var(--text-0);
  }
  ol .current span {
    border-color: var(--frost);
    color: var(--frost);
    box-shadow: 0 0 12px var(--frost-glow);
  }
  ol .done span {
    background: var(--frost);
    border-color: var(--frost);
    color: var(--frost-ink);
  }
  .lead {
    font-size: var(--fs-md);
    max-width: 52ch;
  }
  .found {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    padding: var(--s-4);
  }
  .tick {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: rgba(116, 230, 180, 0.14);
    color: var(--ok);
  }
  .found p {
    font-size: var(--fs-sm);
  }
  .facts {
    display: grid;
    gap: var(--s-3);
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--text-1);
    font-size: var(--fs-sm);
  }
  .facts li {
    display: flex;
    gap: var(--s-3);
    align-items: center;
  }
  .facts :global(svg) {
    color: var(--frost);
  }
  .fine {
    font-size: var(--fs-xs);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    margin-top: var(--s-3);
  }
  .grow {
    flex: 1;
  }
</style>
