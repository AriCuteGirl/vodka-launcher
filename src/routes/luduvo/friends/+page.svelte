<script lang="ts">
  import { api } from "$lib/api";
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { matches } from "$lib/format";
  import { answerLuduvoRequest, joinLuduvoFriend, loadSocial, luduvo, luduvoSignIn, startLuduvo } from "$lib/luduvo.svelte";
  import { app, toast } from "$lib/state.svelte";
  import { toAppError, type LuduvoFriend } from "$lib/types";
  import { onMount } from "svelte";

  // All your Luduvo friends: add someone, answer requests, join a friend, or remove one.
  onMount(startLuduvo);

  let username = $state("");
  let adding = $state(false);
  let removing = $state<LuduvoFriend | null>(null);
  let busy = $state(false);

  const all = $derived(luduvo.friends.status === "ready" ? luduvo.friends.data.filter((f) => matches(`${f.displayName} ${f.name}`, app.search)) : []);
  const groups = $derived(
    [
      { title: "In a game", people: all.filter((f) => f.state === "in_game") },
      { title: "Online", people: all.filter((f) => f.state === "online") },
      { title: "Offline", people: all.filter((f) => f.state !== "in_game" && f.state !== "online") },
    ].filter((g) => g.people.length),
  );

  async function add(e: SubmitEvent) {
    e.preventDefault();
    const name = username.trim().replace(/^@/, "");
    if (!name) return;
    adding = true;
    try {
      await api.luduvoAddFriend(name);
      toast(`Friend request sent to @${name}.`, "ok");
      username = "";
    } catch (err) {
      toast(toAppError(err).message, "danger");
    }
    adding = false;
  }

  async function remove() {
    if (!removing) return;
    busy = true;
    const who = removing;
    try {
      await api.luduvoRemoveFriend(who.id);
      toast(`${who.displayName} is no longer your friend.`, "ok");
      removing = null;
      void loadSocial();
    } catch (err) {
      toast(toAppError(err).message, "danger");
    }
    busy = false;
  }

  const status = (f: LuduvoFriend) =>
    f.state === "in_game" ? (f.placeTitle ? `Playing ${f.placeTitle}` : "In a game") : f.state === "online" ? "Online" : "Offline";
</script>

<div class="friends">
  <h1>Friends</h1>

  {#if !luduvo.user}
    {#if luduvo.started}
      <StateCard icon="lock" title="Sign in to Luduvo to see your friends">
        {#snippet actions()}<Button variant="primary" busy={luduvo.signingIn} onclick={luduvoSignIn}>Sign in to Luduvo</Button>{/snippet}
      </StateCard>
    {/if}
  {:else}
    <form class="add glass" onsubmit={add}>
      <Icon name="plus" size={18} />
      <input bind:value={username} placeholder="Add a friend by username" spellcheck="false" autocomplete="off" maxlength="40" aria-label="Username" />
      <Button type="submit" variant="primary" size="sm" busy={adding} disabled={!username.trim()}>Send request</Button>
    </form>

    {#if luduvo.requests.length}
      <section>
        <h2>Friend requests <span class="count">{luduvo.requests.length}</span></h2>
        <ul class="cards wide">
          {#each luduvo.requests as r (r.id)}
            <li class="card glass">
              <Avatar name={r.displayName} src={r.avatarUrl} size={44} />
              <div class="who"><b>{r.displayName}</b><span class="faint">@{r.name}</span></div>
              <div class="actions">
                <Button size="sm" variant="primary" icon="check" onclick={() => answerLuduvoRequest(r, true)}>Accept</Button>
                <Button size="sm" onclick={() => answerLuduvoRequest(r, false)}>Decline</Button>
              </div>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if luduvo.friends.status === "loading"}
      <ul class="cards">{#each Array(6) as _, i (i)}<li class="card glass"><Skeleton width="44px" height="44px" radius="50%" /><Skeleton width="50%" height="13px" /></li>{/each}</ul>
    {:else if luduvo.friends.status === "error"}
      <StateCard icon="alert" tone="danger" title="Couldn't load your friends">
        {luduvo.friends.error.message}
        {#snippet actions()}<Button icon="refresh" onclick={loadSocial}>Retry</Button>{/snippet}
      </StateCard>
    {:else if luduvo.friends.data.length === 0}
      <StateCard icon="friends" title="No Luduvo friends yet">Send someone a request above. They show up here once they accept.</StateCard>
    {:else if all.length === 0}
      <StateCard icon="search" title="Nobody matches “{app.search}”">Clear the search to see everyone.</StateCard>
    {:else}
      {#each groups as group (group.title)}
        <section>
          <h2>{group.title} <span class="count">{group.people.length}</span></h2>
          <ul class="cards">
            {#each group.people as f (f.id)}
              <li class="card glass" class:dim={f.state !== "in_game" && f.state !== "online"}>
                <a class="open" href="/luduvo/users/{f.id}" title="Open {f.displayName}'s profile">
                  <Avatar name={f.displayName} src={f.avatarUrl} size={44} status={f.state === "in_game" ? "playing" : f.state === "online" ? "online" : undefined} />
                  <div class="who">
                    <b>{f.displayName}</b>
                    <span class="faint">@{f.name}</span>
                    <span class="status" class:live={f.state === "in_game" || f.state === "online"}>{status(f)}</span>
                  </div>
                </a>
                <div class="actions">
                  {#if f.state === "in_game" && f.joinable}
                    <Button size="sm" variant="primary" icon="play" disabled={!!luduvo.playing} onclick={() => joinLuduvoFriend(f)}>Join</Button>
                  {/if}
                  <button class="x" onclick={() => (removing = f)} aria-label="Remove {f.displayName}" title="Remove friend"><Icon name="close" size={15} /></button>
                </div>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    {/if}
  {/if}
</div>

{#if removing}
  <Modal title="Remove {removing.displayName}?" onclose={() => (removing = null)}>
    <p class="muted">You and @{removing.name} won't be Luduvo friends anymore. You can send a new request later.</p>
    {#snippet actions()}
      <Button onclick={() => (removing = null)}>Cancel</Button>
      <Button variant="danger" busy={busy} onclick={remove}>Remove friend</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  .friends {
    display: grid;
    gap: var(--s-5);
    align-content: start;
    padding-right: var(--s-4);
  }
  .add {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: 6px 6px 6px var(--s-4);
    color: var(--text-2);
  }
  .add input {
    flex: 1;
    min-width: 0;
    height: 34px;
    border: 0;
    background: none;
    color: var(--text-0);
    font-size: var(--fs-md);
    outline: none;
    user-select: text;
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
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--glass-hover);
    color: var(--text-1);
    font-weight: 500;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--s-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  /* Requests carry two buttons, so they need room for the name too. */
  .cards.wide {
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
  }
  .card {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3);
    border-radius: var(--r-md);
  }
  .card.dim {
    opacity: 0.6;
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
    flex: 1;
    display: grid;
    min-width: 0;
    line-height: 1.3;
  }
  .who b,
  .who span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    align-items: center;
    gap: var(--s-2);
  }
  .x {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .x:hover {
    background: var(--glass-hover);
    color: var(--danger);
  }
</style>
