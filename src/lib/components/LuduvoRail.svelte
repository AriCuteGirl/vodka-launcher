<script lang="ts">
  import { answerLuduvoRequest, joinLuduvoFriend, loadSocial, luduvo, luduvoSignIn } from "$lib/luduvo.svelte";
  import type { LuduvoFriend } from "$lib/types";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";
  import Skeleton from "./Skeleton.svelte";

  // Luduvo's right-hand column: requests on top, then friends with what they're doing.
  let showOffline = $state(false);
  let refreshing = $state(false);

  const all = $derived(luduvo.friends.status === "ready" ? luduvo.friends.data : []);
  const around = $derived(all.filter((f) => f.state !== "offline"));
  const shown = $derived(showOffline ? all : around);

  const status = (f: LuduvoFriend) => (f.state === "in_game" ? "playing" : f.state === "online" ? "online" : undefined);
  const doing = (f: LuduvoFriend) =>
    f.state === "in_game" ? (f.placeTitle ? `Playing ${f.placeTitle}` : "In a game") : f.state === "online" ? "Online" : "Offline";

  async function refresh() {
    refreshing = true;
    await loadSocial().finally(() => (refreshing = false));
  }
</script>

<aside class="rail" aria-label="Luduvo friends">
  {#if luduvo.requests.length}
    <section class="glass requests">
      <header><h2>Friend requests <span class="pill">{luduvo.requests.length}</span></h2></header>
      <ul class="list">
        {#each luduvo.requests as r (r.id)}
          <li class="friend">
            <Avatar name={r.displayName} src={r.avatarUrl} size={30} />
            <div class="who"><b>{r.displayName}</b><span>@{r.name}</span></div>
            <button class="mini ok" onclick={() => answerLuduvoRequest(r, true)} aria-label="Accept {r.displayName}"><Icon name="check" size={14} /></button>
            <button class="mini" onclick={() => answerLuduvoRequest(r, false)} aria-label="Decline {r.displayName}"><Icon name="close" size={14} /></button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <section class="glass friends">
    <header>
      <h2>Friends {#if around.length}<span class="pill">{around.length} around</span>{/if}</h2>
      {#if luduvo.user}
        <button class="icon" class:spin={refreshing} onclick={refresh} aria-label="Refresh friends"><Icon name="refresh" size={16} /></button>
      {/if}
    </header>

    {#if !luduvo.user}
      <div class="soon">
        <Icon name="friends" size={26} />
        <p>Sign in to Luduvo to see who's online.</p>
        {#if luduvo.started}<button class="mini" onclick={luduvoSignIn}>Sign in</button>{/if}
      </div>
    {:else if luduvo.friends.status === "loading"}
      <div class="list" aria-busy="true">
        {#each Array(4) as _, i (i)}<div class="friend"><Skeleton width="30px" height="30px" radius="50%" /><Skeleton width="60%" height="12px" /></div>{/each}
      </div>
    {:else if luduvo.friends.status === "error"}
      <div class="soon"><Icon name="alert" size={26} /><p>{luduvo.friends.error.message}</p><button class="mini" onclick={refresh}>Retry</button></div>
    {:else if shown.length === 0}
      <div class="soon">
        <Icon name="friends" size={26} />
        <p>{all.length ? "Nobody's online right now." : "No Luduvo friends yet. Add people on luduvo.com."}</p>
      </div>
    {:else}
      <ul class="list">
        {#each shown as f (f.id)}
          <li class="friend" class:offline={f.state === "offline"}>
            <a class="open" href="/luduvo/users/{f.id}" title="Open {f.displayName}'s profile">
              <Avatar name={f.displayName} src={f.avatarUrl} size={30} status={status(f)} />
              <div class="who">
                <b>{f.displayName}</b>
                <span title={doing(f)}>{doing(f)}</span>
              </div>
            </a>
            {#if f.state === "in_game" && f.joinable}
              <button class="mini" disabled={!!luduvo.playing} onclick={() => joinLuduvoFriend(f)} title={luduvo.playing ? "Close your game first" : `Join ${f.displayName}`}>Join</button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    {#if all.length > around.length}
      <button class="toggle" onclick={() => (showOffline = !showOffline)}>
        {showOffline ? "Hide offline friends" : `Show ${all.length - around.length} offline`}
      </button>
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
  h2 .pill {
    margin-left: 6px;
    text-transform: none;
    letter-spacing: 0;
  }
  .pill {
    font-size: 0.6875rem;
    padding: 1px 8px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    color: var(--text-2);
  }
  .requests {
    border-color: rgba(255, 180, 188, 0.3);
  }
  .friends {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 220px;
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
  .friend.offline {
    opacity: 0.55;
  }
  .open {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    flex: 1;
    min-width: 0;
    color: inherit;
    text-decoration: none;
  }
  .open:hover b {
    color: var(--frost);
  }
  .who {
    display: grid;
    flex: 1;
    min-width: 0;
    line-height: 1.25;
  }
  .who b {
    font-size: var(--fs-sm);
    font-weight: 600;
  }
  .who b,
  .who span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .who span {
    font-size: var(--fs-xs);
    color: var(--text-2);
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
  .mini:hover:not(:disabled) {
    background: var(--glass-hover);
    border-color: var(--line-strong);
  }
  .mini:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .mini.ok {
    color: var(--ok);
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
  .toggle {
    margin-top: var(--s-3);
    padding: 6px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: var(--text-2);
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
  }
  .toggle:hover {
    color: var(--text-0);
    background: var(--glass-hover);
  }
</style>
