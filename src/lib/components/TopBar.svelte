<script lang="ts">
  import { page } from "$app/state";
  import { app, removeAccount, signIn, switchAccount } from "$lib/state.svelte";
  import Avatar from "./Avatar.svelte";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import Logo from "./Logo.svelte";

  let menuOpen = $state(false);
  let searchInput: HTMLInputElement;

  const TITLES: Record<string, string> = {
    "": "Home",
    games: "Games",
    players: "Players",
    marketplace: "Marketplace",
    friends: "Friends",
    settings: "Settings",
    gallery: "Design system",
  };
  const crumb = $derived(TITLES[page.url.pathname.split("/")[1] ?? ""] ?? "Home");

  function onKey(e: KeyboardEvent) {
    if ((e.ctrlKey && e.key === "k") || (e.key === "/" && !(e.target instanceof HTMLInputElement))) {
      e.preventDefault();
      searchInput.focus();
    }
    if (e.key === "Escape") menuOpen = false;
  }
</script>

<svelte:window onkeydown={onKey} onclick={() => (menuOpen = false)} />

<header class="topbar">
  <a class="brand" href="/" aria-label="Vodka Launcher home"><Logo size={22} /><span>Vodka</span></a>
  <div class="nav">
    <button onclick={() => history.back()} aria-label="Back"><Icon name="back" size={16} /></button>
    <button onclick={() => history.forward()} aria-label="Forward"><Icon name="forward" size={16} /></button>
  </div>
  <span class="crumb">{crumb}</span>

  <div class="right">
    <label class="search">
      <Icon name="search" size={15} />
      <span class="sr-only">Search your games</span>
      <input bind:this={searchInput} bind:value={app.search} type="search" placeholder="Search your games" spellcheck="false" autocomplete="off" />
      <kbd>Ctrl K</kbd>
    </label>

    <div class="account">
      {#if app.account}
        <button
          class="chip"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onclick={(e) => {
            e.stopPropagation();
            menuOpen = !menuOpen;
          }}
        >
          <Avatar name={app.account.displayName} src={app.account.avatarUrl} size={22} />
          <b>{app.account.displayName}</b>
          <Icon name="chevron" size={14} />
        </button>
        {#if menuOpen}
          <div class="menu" role="menu">
            <p>Accounts</p>
            {#each app.accounts as a (a.id)}
              <button role="menuitemradio" aria-checked={a.active} onclick={() => switchAccount(a.id)}>
                <Avatar name={a.displayName} src={a.avatarUrl} size={22} />
                <span class="who">{a.displayName}<small>@{a.name}</small></span>
                {#if a.active}<Icon name="check" size={16} />{:else if a.inVodkaGame}<em>In game</em>{/if}
              </button>
            {/each}
            <hr />
            <button role="menuitem" onclick={signIn}><Icon name="plus" size={16} /> Add another account</button>
            <a role="menuitem" href="/players"><Icon name="players" size={16} /> Manage accounts</a>
            <button role="menuitem" onclick={() => app.account && removeAccount(app.account.id)}>
              <Icon name="signOut" size={16} /> Sign out {app.account.displayName}
            </button>
          </div>
        {/if}
      {:else if app.booted}
        <Button variant="primary" size="sm" busy={app.signingIn} onclick={signIn}>Sign in</Button>
      {/if}
    </div>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    height: var(--topbar-h);
    padding: 0 var(--s-3) 0 var(--s-4);
    z-index: 6;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: var(--s-2);
    color: var(--text-0);
    font-size: var(--fs-sm);
    font-weight: 650;
    letter-spacing: 0.01em;
    text-decoration: none;
    border-radius: var(--r-sm);
  }
  .nav {
    display: flex;
    gap: 2px;
  }
  .nav button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .nav button:hover {
    background: var(--glass-hover);
    color: var(--text-0);
  }
  .crumb {
    font-size: var(--fs-sm);
    color: var(--text-1);
  }
  .right {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    margin-left: auto;
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    width: 250px;
    height: 30px;
    padding: 0 var(--s-2) 0 10px;
    border-radius: var(--r-sm);
    background: var(--glass);
    border: 1px solid var(--line);
    color: var(--text-2);
    cursor: text;
    transition:
      border-color var(--fast) var(--ease),
      box-shadow var(--slow) var(--ease),
      width var(--slow) var(--ease);
  }
  .search:focus-within {
    width: 320px;
    border-color: rgba(164, 222, 255, 0.5);
    box-shadow: 0 0 0 3px rgba(98, 198, 255, 0.14);
    color: var(--frost);
  }
  input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text-0);
    font-size: var(--fs-sm);
    outline: none;
    user-select: text;
  }
  input::placeholder {
    color: var(--text-2);
  }
  input::-webkit-search-cancel-button {
    display: none;
  }
  kbd {
    font: inherit;
    font-size: 0.625rem;
    padding: 1px 5px;
    border-radius: 5px;
    border: 1px solid var(--line);
    color: var(--text-2);
    white-space: nowrap;
  }
  .account {
    position: relative;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: 30px;
    padding: 0 8px 0 4px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--glass);
    color: var(--text-2);
    cursor: pointer;
  }
  .chip:hover {
    background: var(--glass-hover);
    border-color: var(--line-strong);
  }
  b {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-0);
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 8px);
    min-width: 270px;
    padding: 6px;
    background: #141b24;
    border: 1px solid var(--line-strong);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-2);
    animation: rise var(--fast) var(--ease);
  }
  .menu p {
    padding: 6px 10px 8px;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .menu hr {
    margin: 6px 4px;
    border: 0;
    border-top: 1px solid var(--line);
  }
  .who {
    display: grid;
    line-height: 1.2;
  }
  .who small {
    font-size: 0.6875rem;
    color: var(--text-2);
  }
  .menu :global(svg:last-child:not(:first-child)) {
    margin-left: auto;
    color: var(--frost);
  }
  .menu a {
    color: inherit;
    text-decoration: none;
  }
  .menu button,
  .menu a {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    width: 100%;
    padding: 9px 10px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    font-size: var(--fs-sm);
    text-align: left;
    cursor: pointer;
  }
  .menu a:hover,
  .menu button:hover:not(:disabled) {
    background: var(--glass-hover);
  }
  .menu button:disabled {
    color: var(--text-2);
    cursor: not-allowed;
  }
  em {
    margin-left: auto;
    font-style: normal;
    font-size: 0.6875rem;
    padding: 1px 7px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
  }
</style>
