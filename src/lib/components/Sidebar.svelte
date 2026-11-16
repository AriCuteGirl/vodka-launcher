<script lang="ts">
  import { page } from "$app/state";
  import { chat } from "$lib/chat.svelte";
  import { gameOf } from "$lib/games";
  import Icon from "./Icon.svelte";

  // Each game has its own pages; the rail on the left picks the game.
  const game = $derived(gameOf(page.url.pathname));
  const items = $derived(game.pages);
  // Remembered between starts; a wide sidebar shows each page's name next to its icon.
  let open = $state(typeof localStorage !== "undefined" && localStorage.getItem("vodka-sidebar") === "open");
  function toggle() {
    open = !open;
    try {
      localStorage.setItem("vodka-sidebar", open ? "open" : "closed");
    } catch {
      // It just won't be remembered.
    }
  }
  // A game's home is only active on itself; other pages also cover what is under them.
  const active = (href: string) => (href === game.home || href === "/" ? page.url.pathname === href : page.url.pathname.startsWith(href));
</script>

<nav class="sidebar" class:open aria-label="{game.name} pages">
  <button class="toggle" onclick={toggle} aria-expanded={open} aria-label={open ? "Collapse the sidebar" : "Expand the sidebar to show page names"}>
    <Icon name="menu" size={19} />
    <span class="text">Menu</span>
  </button>
  <ul>
    {#each items as item (item.href)}
      <li>
        <a href={item.href} class:active={active(item.href)} aria-current={active(item.href) ? "page" : undefined} aria-label={item.label}>
          <Icon name={item.icon} size={19} />
          {#if item.href === "/chat" && chat.unreadTotal > 0}<i class="badge">{chat.unreadTotal > 9 ? "9+" : chat.unreadTotal}</i>{/if}
          <span class="text">{item.label}</span>
          <span class="tip" aria-hidden="true">{item.label}</span>
        </a>
      </li>
    {/each}
  </ul>
  <a href="/settings" class="bottom" class:active={active("/settings")} aria-current={active("/settings") ? "page" : undefined} aria-label="Settings">
    <Icon name="settings" size={19} />
    <span class="text">Settings</span>
    <span class="tip" aria-hidden="true">Settings</span>
  </a>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-4);
    width: var(--sidebar-w);
    padding: var(--s-2) 0 var(--s-4);
    z-index: 5;
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: var(--s-1);
    margin: 0;
    padding: 0;
    list-style: none;
    flex: 1;
  }
  a {
    position: relative;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 10px;
    color: var(--text-2);
    transition:
      color var(--fast) var(--ease),
      background var(--fast) var(--ease);
  }
  a:hover {
    color: var(--text-0);
    background: var(--glass-hover);
  }
  a.active {
    color: var(--frost);
    background: var(--glass-strong);
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  a.active::before {
    content: "";
    position: absolute;
    left: -10px;
    width: 3px;
    height: 16px;
    border-radius: 0 3px 3px 0;
    background: var(--frost);
    box-shadow: 0 0 12px var(--frost-strong);
  }
  .sidebar {
    transition: width var(--slow) var(--ease);
  }
  .sidebar.open {
    width: 200px;
    align-items: stretch;
    padding-left: var(--s-2);
    padding-right: var(--s-2);
  }
  .toggle {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    flex: none;
    border: 0;
    border-radius: 10px;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .toggle:hover {
    color: var(--text-0);
    background: var(--glass-hover);
  }
  .text {
    display: none;
    font-size: var(--fs-sm);
    font-weight: 600;
    white-space: nowrap;
  }
  .open .text {
    display: inline;
  }
  a {
    text-decoration: none;
  }
  .open a,
  .open .toggle {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: var(--s-3);
    width: 100%;
    padding: 0 10px;
  }
  .open .toggle {
    color: var(--text-1);
  }
  /* Names are on screen, so the hover labels would only repeat them. */
  .open .tip {
    display: none;
  }
  .open .badge {
    position: static;
    margin-left: auto;
    order: 3;
  }
  .open a.active::before {
    left: calc(var(--s-2) * -1);
  }
  .badge {
    position: absolute;
    top: 2px;
    right: 2px;
    min-width: 16px;
    padding: 0 4px;
    border-radius: var(--r-full);
    background: var(--frost);
    color: var(--frost-ink);
    font-size: 0.625rem;
    font-style: normal;
    font-weight: 700;
    line-height: 16px;
    text-align: center;
  }
  .tip {
    position: absolute;
    left: calc(100% + 12px);
    padding: 5px 10px;
    border-radius: var(--r-sm);
    background: #18202a;
    border: 1px solid var(--line-strong);
    color: var(--text-0);
    font-size: var(--fs-xs);
    font-weight: 600;
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transform: translateX(-4px);
    transition:
      opacity var(--fast) var(--ease),
      transform var(--fast) var(--ease);
  }
  a:hover .tip,
  a:focus-visible .tip {
    opacity: 1;
    transform: none;
  }
</style>
