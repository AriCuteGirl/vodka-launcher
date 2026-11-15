<script lang="ts">
  import { elapsed, memory } from "$lib/format";
  import { app, joinFriend, onWindows, refreshMocktail, stopInstance } from "$lib/state.svelte";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";
  import Skeleton from "./Skeleton.svelte";

  let now = $state(Date.now());
  let checking = $state(false);

  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  async function recheck() {
    checking = true;
    await refreshMocktail().finally(() => (checking = false));
  }

  const m = $derived(app.mocktail);
  const around = $derived(app.friends.status === "ready" ? app.friends.data.filter((f) => f.presence.kind !== "offline") : []);
</script>

<aside class="rail" aria-label="Status">
  <section class="glass friends">
    <header>
      <h2>Friends {#if around.length}<span class="pill">{around.length} around</span>{/if}</h2>
      <a href="/friends">View all{#if app.friendRequests.length}<i>{app.friendRequests.length}</i>{/if}</a>
    </header>
    {#if !app.account}
      <div class="soon"><Icon name="friends" size={26} /><p>Sign in to see who's online.</p></div>
    {:else if app.friends.status === "loading"}
      <div class="list" aria-busy="true">
        {#each Array(4) as _, i (i)}<div class="friend"><Skeleton width="30px" height="30px" radius="50%" /><Skeleton width="60%" height="12px" /></div>{/each}
      </div>
    {:else if app.friends.status === "error"}
      <div class="soon"><Icon name="alert" size={26} /><p>{app.friends.error.message}</p></div>
    {:else if around.length === 0}
      <div class="soon">
        <Icon name="friends" size={26} />
        <p>{app.friends.data.length ? "Nobody's online right now." : "No friends on this account yet."}</p>
      </div>
    {:else}
      <ul class="list">
        {#each around.slice(0, 8) as f (f.id)}
          <li class="friend">
            <Avatar name={f.displayName} src={f.avatarUrl} size={30} status={f.presence.kind === "in_game" ? "playing" : "online"} />
            <div class="who">
              <b>{f.displayName}</b>
              <span>{f.presence.kind === "in_game" ? (f.presence.location ?? "In a game") : f.presence.kind === "studio" ? "In Studio" : "Online"}</span>
            </div>
            {#if f.presence.gameId}<button class="mini" onclick={() => joinFriend(f)}>Join</button>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  {#each app.instances as inst (inst.id)}
    <section class="glass session" class:mine={inst.accountId === app.account?.id}>
      <header>
        <h2>{inst.phase === "playing" ? "Now playing" : "Now starting"}</h2>
        <span class="time"><Icon name="clock" size={14} />{elapsed((now - inst.startedAtMs) / 1000)}</span>
      </header>
      <p class="game">{inst.game.name}</p>
      <p class="who faint">{inst.accountName}{#if !onWindows()} · {inst.fps != null ? `${Math.round(inst.fps)} FPS · ` : ""}{Math.round(inst.cpuPercent)}% CPU · {memory(inst.memoryBytes)}{/if}</p>
      {#if inst.phase === "starting"}
        <pre class="selectable" aria-label={onWindows() ? "Game status" : "Mocktail output"}>{inst.log.slice(-12).join("\n") || (onWindows() ? "Waiting for Roblox to report you in the game…" : "Waiting for Mocktail…")}</pre>
      {/if}
      <div class="row">
        {#if !onWindows()}<button class="mini" onclick={() => stopInstance(inst.id, true)}><Icon name="refresh" size={14} />Restart</button>{/if}
        <button class="mini danger" onclick={() => stopInstance(inst.id)}><Icon name="close" size={14} />Close</button>
      </div>
    </section>
  {/each}

  <section class="glass">
    <header>
      <h2>{onWindows() ? "Roblox" : "Mocktail"}</h2>
      <button class="icon" onclick={recheck} aria-label="Check again" class:spin={checking}><Icon name="refresh" size={16} /></button>
    </header>
    {#if !m}
      <p class="faint">Looking for {onWindows() ? "Roblox" : "Mocktail"}…</p>
    {:else if m.installed}
      <p class="status ok"><i></i>{m.running ? "Running" : "Ready"}</p>
      <dl>
        <dt>{onWindows() ? "Runs with" : "Mocktail"}</dt>
        <dd>{m.version ?? "unknown"}{onWindows() ? "" : " · Flatpak"}</dd>
        <dt>Roblox client</dt>
        <dd title={m.robloxVersion ?? undefined}>{m.robloxVersion ?? "downloads on first launch"}</dd>
      </dl>
    {:else}
      <p class="status bad"><i></i>Not installed</p>
      <p class="faint small">Vodka needs {onWindows() ? "the Roblox client" : "Mocktail"} to run games. Setup steps are on Home.</p>
    {/if}
  </section>

</aside>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    gap: var(--s-4);
    width: var(--rail-w);
    padding: var(--s-4) var(--s-4) var(--s-4) 0;
    overflow-y: auto;
  }
  section {
    padding: var(--s-4);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--s-3);
  }
  h2 {
    font-size: var(--fs-sm);
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-1);
  }
  .status {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-weight: 600;
  }
  .status i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
    box-shadow: 0 0 10px var(--ok);
  }
  .status.bad i {
    background: var(--warn);
    box-shadow: 0 0 10px var(--warn);
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px var(--s-3);
    margin: var(--s-3) 0 0;
    font-size: var(--fs-sm);
  }
  dt {
    color: var(--text-2);
  }
  dd {
    margin: 0;
    min-width: 0;
    text-align: right;
    color: var(--text-1);
    /* Windows client versions are long folder names like version-4ea8e85481e34f46. */
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .small {
    font-size: var(--fs-sm);
    margin-top: var(--s-2);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .icon:hover {
    color: var(--text-0);
    background: var(--glass-hover);
  }
  .spin :global(svg) {
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .session.mine {
    border-color: rgba(164, 222, 255, 0.35);
    box-shadow: var(--glow);
  }
  .who {
    font-size: var(--fs-xs);
    margin-bottom: var(--s-3);
    font-variant-numeric: tabular-nums;
  }
  .row {
    display: flex;
    gap: var(--s-2);
  }
  .mini {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--glass);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
  }
  .mini:hover {
    background: var(--glass-hover);
    border-color: var(--line-strong);
  }
  .mini.danger {
    color: var(--danger);
  }
  .time {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-xs);
    color: var(--frost);
    font-variant-numeric: tabular-nums;
  }
  .game {
    font-weight: 600;
  }
  pre {
    margin: 0;
    max-height: 110px;
    margin-bottom: var(--s-3);
    padding: var(--s-2) var(--s-3);
    overflow: auto;
    border-radius: var(--r-sm);
    background: rgba(0, 0, 0, 0.4);
    color: var(--text-1);
    font-size: 0.6875rem;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .friends {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 220px;
  }
  header a {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--text-1);
    text-decoration: none;
  }
  header a:hover {
    color: var(--frost);
  }
  header a i {
    font-style: normal;
    font-weight: 700;
    min-width: 18px;
    padding: 0 5px;
    border-radius: var(--r-full);
    background: var(--frost);
    color: var(--frost-ink);
    text-align: center;
  }
  h2 .pill {
    margin-left: 6px;
    text-transform: none;
    letter-spacing: 0;
  }
  .list {
    display: grid;
    gap: 2px;
    margin: 0 calc(var(--s-2) * -1);
    padding: 0;
    list-style: none;
  }
  .friend {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: 6px var(--s-2);
    border-radius: var(--r-sm);
  }
  li.friend:hover {
    background: var(--glass-hover);
  }
  .friend .who {
    display: grid;
    flex: 1;
    min-width: 0;
    margin: 0;
    line-height: 1.25;
  }
  .friend b {
    font-size: var(--fs-sm);
    font-weight: 600;
  }
  .friend span,
  .friend b {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .friend span {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .soon {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: var(--s-2);
    text-align: center;
    color: var(--text-2);
    font-size: var(--fs-sm);
  }
  .soon p {
    max-width: 22ch;
    color: var(--text-1);
  }
  .pill {
    font-size: 0.6875rem;
    padding: 1px 8px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    color: var(--text-2);
  }
</style>
