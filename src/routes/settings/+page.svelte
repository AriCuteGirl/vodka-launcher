<script lang="ts">
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import TweaksEditor from "$lib/components/TweaksEditor.svelte";
  import { app, askMultiInstance, onWindows, removeAccount, setFlags, signIn } from "$lib/state.svelte";
  import { alerts, savePrefs } from "$lib/alerts.svelte";
  import { version } from "../../../package.json";
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
    <h2>Notifications</h2>
    <label class="row toggle">
      <input type="checkbox" bind:checked={alerts.prefs.online} onchange={savePrefs} />
      <span class="grow"><b>When a friend comes online</b><span class="faint">A small pop-up at the top right, for Roblox and Luduvo friends.</span></span>
    </label>
    <label class="row toggle">
      <input type="checkbox" bind:checked={alerts.prefs.playing} onchange={savePrefs} />
      <span class="grow"><b>When a friend starts a game</b><span class="faint">With a Join button when their server can be joined.</span></span>
    </label>
  </section>

  <section class="glass">
    <h2>Performance</h2>
    <TweaksEditor />
  </section>

  {#if app.mocktail?.engine === "roblox"}
    <section class="glass">
      <h2>Launching</h2>
      <p class="muted">
        On Windows, Vodka starts the official Roblox client signed in as the account you pick, using a one-time ticket from Roblox. Restart, Gather
        and per-game CPU and FPS are Linux features for now.
      </p>
      <div class="row">
        <div class="grow">
          <b>Several games at once</b>
          <p class="faint">
            {app.settings.multiInstanceAccepted
              ? "On. Each account can run its own Roblox window, with a reminder each time. Unofficial: Roblox may block it, and it adds account risk."
              : "Off: one game at a time. Turning it on is unofficial and risky; Vodka explains before it does anything."}
          </p>
        </div>
        {#if app.settings.multiInstanceAccepted}
          <Button onclick={() => setFlags({ multiInstanceAccepted: false })}>Turn off</Button>
        {:else}
          <Button onclick={askMultiInstance}>Turn on…</Button>
        {/if}
      </div>
    </section>
  {:else}
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
  {/if}

  <section class="glass about">
    <h2>About</h2>
    <div class="row">
      <Logo size={44} />
      <div class="grow">
        <b>Vodka Launcher {version}</b>
        <p class="faint">A Roblox launcher{onWindows() ? "" : " for Linux"}. Apache-2.0 licensed.</p>
      </div>
      <Button variant="ghost" onclick={() => setFlags({ onboarded: false })}>Run setup again</Button>
    </div>
    {#if onWindows()}
      <p class="muted">On Windows, Vodka starts the official Roblox client you installed from roblox.com. The client is not part of Vodka.</p>
    {:else}
      <p class="muted">
        Vodka runs Roblox through <b>Mocktail</b>, an Apache-2.0 compatibility runtime by the Mocktail Project Authors. Vodka starts Mocktail as a
        separate program and does not include or modify it. The Roblox client is downloaded by Mocktail and is not part of Vodka.
      </p>
    {/if}
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
  .toggle {
    cursor: pointer;
  }
  .toggle input {
    width: 18px;
    height: 18px;
    accent-color: var(--frost-strong);
  }
  .toggle .grow {
    display: grid;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  p {
    font-size: var(--fs-sm);
  }
</style>
