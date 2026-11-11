<script lang="ts">
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import { app, saveSettings, signIn, signOut } from "$lib/state.svelte";
</script>

<div class="settings">
  <h1>Settings</h1>

  <section class="glass">
    <h2>Account</h2>
    {#if app.account}
      <div class="row">
        <Avatar name={app.account.displayName} src={app.account.avatarUrl} size={40} />
        <div class="grow">
          <b>{app.account.displayName}</b>
          <p class="faint">@{app.account.name} · session stored in the system keyring</p>
        </div>
        <Button variant="danger" icon="signOut" onclick={signOut}>Sign out</Button>
      </div>
    {:else}
      <div class="row">
        <p class="grow muted">You're not signed in.</p>
        <Button variant="primary" busy={app.signingIn} onclick={signIn}>Sign in with Roblox</Button>
      </div>
    {/if}
  </section>

  <section class="glass">
    <h2>Launching</h2>
    <label class="row toggle">
      <div class="grow">
        <b>Start Mocktail signed in as this account <span class="tag">Experimental</span></b>
        <p class="faint">
          Hands your session to Mocktail when a game starts, so the game opens as the account shown in Vodka. Mocktail then keeps its own copy in a
          private file. Leave this off to use whichever account Mocktail is already signed in to.
        </p>
      </div>
      <input
        type="checkbox"
        role="switch"
        checked={app.settings.shareSessionWithMocktail}
        onchange={(e) => saveSettings({ shareSessionWithMocktail: e.currentTarget.checked })}
      />
    </label>
  </section>

  <section class="glass about">
    <h2>About</h2>
    <div class="row">
      <Logo size={44} />
      <div class="grow">
        <b>Vodka Launcher 0.1.0</b>
        <p class="faint">A Roblox launcher for Linux. Apache-2.0 licensed.</p>
      </div>
      <Button variant="ghost" onclick={() => saveSettings({ onboarded: false })}>Run setup again</Button>
    </div>
    <p class="muted">
      Vodka runs Roblox through <b>Mocktail</b>, an Apache-2.0 compatibility runtime by the Mocktail Project Authors. Vodka starts Mocktail as a
      separate program and does not include or modify it. The Roblox client is downloaded by Mocktail and is not part of Vodka.
    </p>
    <p class="faint">
      Vodka Launcher is an independent project. It is not affiliated with, endorsed by, or sponsored by Roblox Corporation, Mocktail, or Lunar
      Client. Roblox is a trademark of Roblox Corporation.
    </p>
    {#if import.meta.env.DEV}<a href="/gallery">Design system gallery</a>{/if}
  </section>
</div>

<style>
  .settings {
    display: grid;
    gap: var(--s-5);
    max-width: 760px;
  }
  section {
    display: grid;
    gap: var(--s-4);
    padding: var(--s-5);
  }
  h2 {
    font-size: var(--fs-sm);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-1);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--s-4);
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  p {
    font-size: var(--fs-sm);
  }
  .tag {
    margin-left: 6px;
    font-size: 0.6875rem;
    font-weight: 600;
    padding: 1px 8px;
    border-radius: var(--r-full);
    border: 1px solid rgba(255, 213, 133, 0.4);
    color: var(--warn);
  }
  .toggle {
    cursor: pointer;
  }
  input[role="switch"] {
    appearance: none;
    flex: none;
    width: 44px;
    height: 26px;
    margin: 0;
    border-radius: var(--r-full);
    background: var(--glass-strong);
    border: 1px solid var(--line-strong);
    cursor: pointer;
    position: relative;
    transition: background var(--fast) var(--ease);
  }
  input[role="switch"]::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--text-1);
    transition:
      transform var(--slow) var(--ease),
      background var(--fast) var(--ease);
  }
  input[role="switch"]:checked {
    background: var(--frost-strong);
    border-color: var(--frost);
  }
  input[role="switch"]:checked::after {
    transform: translateX(18px);
    background: var(--frost-ink);
  }
</style>
