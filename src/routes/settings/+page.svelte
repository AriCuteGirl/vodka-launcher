<script lang="ts">
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import TweaksEditor from "$lib/components/TweaksEditor.svelte";
  import { app, removeAccount, setFlags, signIn } from "$lib/state.svelte";
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
        <a href="/players">Manage accounts</a>
        <Button variant="danger" icon="signOut" onclick={() => app.account && removeAccount(app.account.id)}>Sign out</Button>
      </div>
    {:else}
      <div class="row">
        <p class="grow muted">You're not signed in.</p>
        <Button variant="primary" busy={app.signingIn} onclick={signIn}>Sign in with Roblox</Button>
      </div>
    {/if}
  </section>

  <section class="glass">
    <h2>Performance</h2>
    <TweaksEditor />
  </section>

  <section class="glass">
    <h2>Launching</h2>
    <p class="muted">
      Each account plays in its own Mocktail profile, so accounts never share saved data. Vodka hands the account's session to Mocktail when a game
      starts and deletes Mocktail's copy when the game closes.
    </p>
    <div class="row">
      <div class="grow">
        <b>Several games at once</b>
        <p class="faint">
          {app.settings.multiInstanceAccepted ? "You accepted the warning. Vodka starts further games without asking, with a short reminder each time." : "Vodka asks before the first time."}
        </p>
      </div>
      {#if app.settings.multiInstanceAccepted}
        <Button onclick={() => setFlags({ multiInstanceAccepted: false })}>Ask me again</Button>
      {/if}
    </div>
  </section>

  <section class="glass about">
    <h2>About</h2>
    <div class="row">
      <Logo size={44} />
      <div class="grow">
        <b>Vodka Launcher 0.1.0</b>
        <p class="faint">A Roblox launcher for Linux. Apache-2.0 licensed.</p>
      </div>
      <Button variant="ghost" onclick={() => setFlags({ onboarded: false })}>Run setup again</Button>
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
    max-width: 860px;
    padding-right: var(--s-4);
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
</style>
