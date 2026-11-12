<script lang="ts">
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { matches } from "$lib/format";
  import { answerFriendRequest, app, joinFriend, refreshFriends, signIn } from "$lib/state.svelte";
  import type { Person } from "$lib/types";
  import { onMount } from "svelte";

  let filter = $state("");
  let answering = $state<number | null>(null);

  onMount(() => {
    void refreshFriends(app.friends.status === "ready");
    // Fresher than the rail while this page is open.
    const timer = setInterval(() => void refreshFriends(true), 30_000);
    return () => clearInterval(timer);
  });

  const all = $derived(app.friends.status === "ready" ? app.friends.data : []);
  const shown = $derived(all.filter((f) => matches(`${f.displayName} ${f.name}`, filter)));
  const groups = $derived(
    [
      { title: "In a game", people: shown.filter((f) => f.presence.kind === "in_game") },
      { title: "Online", people: shown.filter((f) => f.presence.kind === "online" || f.presence.kind === "studio") },
      { title: "Offline", people: shown.filter((f) => f.presence.kind === "offline") },
    ].filter((g) => g.people.length),
  );

  function status(f: Person): string {
    switch (f.presence.kind) {
      case "in_game":
        return f.presence.location ? `Playing ${f.presence.location}` : "In a game";
      case "studio":
        return "In Roblox Studio";
      case "online":
        return "Online";
      default:
        return "Offline";
    }
  }

  async function answer(person: Person, accept: boolean) {
    answering = person.id;
    await answerFriendRequest(person, accept);
    answering = null;
  }
</script>

<div class="friends">
  <header class="top">
    <div>
      <h1>Friends</h1>
      <p class="muted">
        {#if app.account}{app.account.displayName}'s friends on Roblox{#if all.length}, {all.length} in total{/if}.{:else}See who's online and join them.{/if}
      </p>
    </div>
    {#if all.length > 6}
      <label class="search">
        <Icon name="search" size={16} />
        <span class="sr-only">Filter friends</span>
        <input bind:value={filter} type="search" placeholder="Filter friends" spellcheck="false" autocomplete="off" />
      </label>
    {/if}
  </header>

  {#if !app.account}
    <StateCard icon="lock" title="Sign in to see your friends">
      Vodka reads your friends list from Roblox with the account you choose.
      {#snippet actions()}<Button variant="primary" busy={app.signingIn} onclick={signIn}>Sign in</Button>{/snippet}
    </StateCard>
  {:else}
    {#if app.friendRequests.length}
      <section>
        <h2>Friend requests <span class="count">{app.friendRequests.length}</span></h2>
        <ul class="cards">
          {#each app.friendRequests as r (r.id)}
            <li class="card request glass">
              <Avatar name={r.displayName} src={r.avatarUrl} size={44} />
              <div class="who"><b>{r.displayName}</b><span class="faint">@{r.name}</span></div>
              <div class="actions">
                <Button size="sm" variant="primary" icon="check" busy={answering === r.id} onclick={() => answer(r, true)}>Accept</Button>
                <Button size="sm" variant="ghost" disabled={answering === r.id} onclick={() => answer(r, false)}>Decline</Button>
              </div>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if app.friends.status === "loading"}
      <ul class="cards" aria-busy="true" aria-label="Loading friends">
        {#each Array(6) as _, i (i)}
          <li class="card glass"><Skeleton width="44px" height="44px" radius="50%" /><div class="who"><Skeleton width="70%" height="13px" /><Skeleton width="45%" height="11px" /></div></li>
        {/each}
      </ul>
    {:else if app.friends.status === "error"}
      {#if app.friends.error.kind === "rate_limited"}
        <StateCard icon="clock" tone="warn" title="Roblox asked us to slow down">
          Too many requests in a short time. Wait a minute, then retry.
          {#snippet actions()}<Button icon="refresh" onclick={() => refreshFriends()}>Retry</Button>{/snippet}
        </StateCard>
      {:else if app.friends.error.kind === "offline"}
        <StateCard icon="wifiOff" tone="warn" title="Can't reach Roblox">
          Check your internet connection.
          {#snippet actions()}<Button icon="refresh" onclick={() => refreshFriends()}>Retry</Button>{/snippet}
        </StateCard>
      {:else if app.friends.error.kind === "session_expired"}
        <StateCard icon="lock" tone="warn" title="Your Roblox session ended">
          Sign in again to see your friends.
          {#snippet actions()}<Button variant="primary" busy={app.signingIn} onclick={signIn}>Sign in</Button>{/snippet}
        </StateCard>
      {:else}
        <StateCard icon="alert" tone="danger" title="Couldn't load your friends">
          {app.friends.error.message}
          {#snippet actions()}<Button icon="refresh" onclick={() => refreshFriends()}>Retry</Button>{/snippet}
        </StateCard>
      {/if}
    {:else if all.length === 0}
      <StateCard icon="friends" title="No friends on this account yet">Add friends on roblox.com and they'll show up here with what they're playing.</StateCard>
    {:else if shown.length === 0}
      <StateCard icon="search" title="No friend matches “{filter}”">Try part of their display name or username.</StateCard>
    {:else}
      {#each groups as group (group.title)}
        <section>
          <h2>{group.title} <span class="count">{group.people.length}</span></h2>
          <ul class="cards">
            {#each group.people as f (f.id)}
              <li class="card glass" class:dim={f.presence.kind === "offline"}>
                <Avatar
                  name={f.displayName}
                  src={f.avatarUrl}
                  size={44}
                  status={f.presence.kind === "in_game" ? "playing" : f.presence.kind === "offline" ? undefined : "online"}
                />
                <div class="who">
                  <b>{f.displayName}</b>
                  <span class="faint">@{f.name}</span>
                  <span class="status" class:live={f.presence.kind !== "offline"}>{status(f)}</span>
                </div>
                {#if f.presence.kind === "in_game"}
                  <div class="actions">
                    {#if f.presence.gameId}
                      <Button size="sm" variant="primary" icon="play" disabled={!app.mocktail?.installed} onclick={() => joinFriend(f)}>Join</Button>
                    {:else}
                      <span class="faint private" title="Their privacy settings don't share which server they're on">Can't be joined</span>
                    {/if}
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    {/if}
  {/if}
</div>

<style>
  .friends {
    display: grid;
    gap: var(--s-5);
    align-content: start;
    padding-right: var(--s-4);
  }
  .top {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--s-4);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    width: 240px;
    height: 36px;
    padding: 0 var(--s-3);
    border-radius: var(--r-sm);
    background: var(--glass);
    border: 1px solid var(--line);
    color: var(--text-2);
  }
  .search:focus-within {
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
  input::-webkit-search-cancel-button {
    display: none;
  }
  section {
    display: grid;
    gap: var(--s-3);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-size: var(--fs-md);
  }
  .count {
    font-size: var(--fs-xs);
    font-weight: 500;
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--glass-hover);
    color: var(--text-1);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
    gap: var(--s-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .card {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3);
    border-radius: var(--r-md);
  }
  /* Requests put their buttons on a second line so long names stay readable. */
  .request {
    flex-wrap: wrap;
  }
  .request .who {
    flex-basis: 120px;
  }
  .request .actions {
    flex-basis: 100%;
    justify-content: flex-end;
  }
  .card.dim {
    opacity: 0.7;
  }
  .who {
    display: grid;
    flex: 1;
    min-width: 0;
    gap: 1px;
    line-height: 1.3;
  }
  .who b,
  .who span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .who span {
    font-size: var(--fs-xs);
  }
  .status {
    color: var(--text-2);
  }
  .status.live {
    color: var(--ok);
  }
  .actions {
    display: flex;
    gap: 4px;
    flex: none;
  }
  .private {
    font-size: var(--fs-xs);
  }
</style>
