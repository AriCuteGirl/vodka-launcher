<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { api } from "$lib/api";
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import GameRow from "$lib/components/GameRow.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { compact } from "$lib/format";
  import { app, joinFriend } from "$lib/state.svelte";
  import { toAppError, type Game, type Loadable, type Person, type Profile, type UserGroup } from "$lib/types";

  // Someone's Roblox profile: who they are, what they're doing (with Join), their friends, games and groups.
  const userId = $derived(Number(page.params.id));

  let profile = $state<Loadable<Profile>>({ status: "loading" });
  let friends = $state<Loadable<Person[]>>({ status: "loading" });
  let made = $state<Loadable<Game[]>>({ status: "loading" });
  let favorites = $state<Loadable<Game[]>>({ status: "loading" });
  let groups = $state<Loadable<UserGroup[]>>({ status: "loading" });
  let avatarFailed = $state(false);

  async function load<T>(set: (v: Loadable<T>) => void, get: () => Promise<T>) {
    set({ status: "loading" });
    try {
      set({ status: "ready", data: await get() });
    } catch (e) {
      set({ status: "error", error: toAppError(e) });
    }
  }

  $effect(() => {
    const id = userId;
    avatarFailed = false;
    void load((v) => (profile = v), () => api.userProfile(id));
    void load((v) => (made = v), () => api.userGames(id));
    void load((v) => (groups = v), () => api.userGroups(id));
    if (app.account) {
      void load((v) => (friends = v), () => api.userFriends(id));
      void load((v) => (favorites = v), () => api.userFavorites(id));
    }
  });

  const p = $derived(profile.status === "ready" ? profile.data : null);
  const me = $derived(app.account?.id === userId);
  const isFriend = $derived(app.friends.status === "ready" && app.friends.data.some((f) => f.id === userId));
  const presence = $derived(p?.presence ?? null);
  const status = $derived.by(() => {
    if (!presence) return null;
    if (presence.kind === "in_game") return { text: presence.location ? `Playing ${presence.location}` : "In a game", tone: "playing" };
    if (presence.kind === "studio") return { text: "In Roblox Studio", tone: "online" };
    if (presence.kind === "online") return { text: "Online", tone: "online" };
    return { text: "Offline", tone: "offline" };
  });
  // Joining goes through the same path as the friends list, which needs a friend-shaped person.
  const asPerson = $derived<Person | null>(p && presence ? { id: p.id, name: p.name, displayName: p.displayName, avatarUrl: p.avatarUrl, presence } : null);
  const joined = $derived(p?.created ? new Date(p.created).toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" }) : null);
  const friendList = $derived(friends.status === "ready" ? friends.data : []);
</script>

{#if profile.status === "error"}
  <div class="pad">
    <StateCard icon="alert" tone="danger" title="Couldn't open this profile">
      {profile.error.message}
      {#snippet actions()}<Button onclick={() => history.back()}>Back</Button>{/snippet}
    </StateCard>
  </div>
{:else}
  <article class="profile">
    <button class="back" onclick={() => history.back()} aria-label="Back"><Icon name="back" size={18} /></button>

    <header class="top glass">
      <div class="figure">
        {#if p?.avatarUrl && !avatarFailed}
          <img src={p.avatarUrl} alt="{p.displayName}'s avatar" onerror={() => (avatarFailed = true)} />
        {:else if p}
          <span class="initial">{p.displayName.slice(0, 1).toUpperCase()}</span>
        {:else}
          <Skeleton height="100%" radius="var(--r-lg)" />
        {/if}
      </div>
      <div class="info">
        {#if p}
          <h1>{p.displayName}{#if p.verified}<span class="check" title="Verified"><Icon name="check" size={13} /></span>{/if}</h1>
          <p class="handle">@{p.name}{#if me}<span class="tag">You</span>{:else if isFriend}<span class="tag">Friend</span>{/if}{#if p.banned}<span class="tag bad">Banned</span>{/if}</p>
          {#if status}
            <p class="status {status.tone}"><i></i>{status.text}</p>
          {/if}
          <dl class="counts">
            <div><dt>Friends</dt><dd>{p.friends != null ? compact(p.friends) : "?"}</dd></div>
            <div><dt>Followers</dt><dd>{p.followers != null ? compact(p.followers) : "?"}</dd></div>
            <div><dt>Following</dt><dd>{p.following != null ? compact(p.following) : "?"}</dd></div>
          </dl>
          <div class="actions">
            {#if presence?.kind === "in_game" && asPerson && !me}
              {#if presence.gameId}
                <Button variant="primary" size="lg" icon="play" disabled={!app.mocktail?.installed} onclick={() => asPerson && joinFriend(asPerson)}>Join {p.displayName}</Button>
              {:else}
                <span class="faint" title="Their privacy settings don't share which server they're on">Their server can't be joined</span>
              {/if}
            {/if}
            {#if presence?.universeId}
              <Button size="lg" icon="games" onclick={() => goto(`/games/${presence?.universeId}`)}>Open the game</Button>
            {/if}
          </div>
          {#if joined}<p class="faint since">On Roblox since {joined}</p>{/if}
        {:else}
          <Skeleton width="60%" height="32px" />
          <Skeleton width="30%" height="14px" />
          <Skeleton width="80%" height="56px" />
        {/if}
      </div>
    </header>

    {#if p?.description}
      <section class="section">
        <h2>About</h2>
        <p class="about selectable">{p.description}</p>
      </section>
    {/if}

    {#if app.account}
      <section class="section">
        <h2>Friends {#if friendList.length}<span class="count">{friendList.length}</span>{/if}</h2>
        {#if friends.status === "loading"}
          <div class="people">{#each Array(8) as _, i (i)}<Skeleton width="72px" height="92px" radius="var(--r-md)" />{/each}</div>
        {:else if friends.status === "error"}
          <p class="faint">{friends.error.message}</p>
        {:else if friendList.length === 0}
          <p class="faint">{me ? "You haven't added anyone yet." : "No friends to show. They may hide their friends list."}</p>
        {:else}
          <ul class="people">
            {#each friendList.slice(0, 60) as f (f.id)}
              <li>
                <a href="/users/{f.id}" title="{f.displayName} (@{f.name})">
                  <Avatar
                    name={f.displayName}
                    src={f.avatarUrl}
                    size={56}
                    status={f.presence.kind === "in_game" ? "playing" : f.presence.kind === "offline" ? undefined : "online"}
                  />
                  <span>{f.displayName}</span>
                </a>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}

    <GameRow
      title="Games {p ? p.displayName : 'they'} made"
      icon="games"
      row={made}
      filter={false}
      onretry={() => load((v) => (made = v), () => api.userGames(userId))}
      emptyTitle="No public games"
      emptyText="They haven't published any games, or keep them private."
    />

    {#if app.account}
      <GameRow
        title="Favorites"
        icon="star"
        row={favorites}
        filter={false}
        onretry={() => load((v) => (favorites = v), () => api.userFavorites(userId))}
        emptyTitle="No favorites to show"
        emptyText="They haven't favorited any games, or keep them private."
      />
    {/if}

    <section class="section">
      <h2>Groups {#if groups.status === "ready" && groups.data.length}<span class="count">{groups.data.length}</span>{/if}</h2>
      {#if groups.status === "loading"}
        <Skeleton height="64px" />
      {:else if groups.status === "error"}
        <p class="faint">{groups.error.message}</p>
      {:else if groups.data.length === 0}
        <p class="faint">Not in any groups.</p>
      {:else}
        <ul class="groups">
          {#each groups.data as g (g.id)}
            <li class="glass">
              {#if g.iconUrl}<img src={g.iconUrl} alt="" />{:else}<span class="gicon"><Icon name="friends" size={20} /></span>{/if}
              <div>
                <b>{g.name}{#if g.verified}<span class="check small"><Icon name="check" size={10} /></span>{/if}</b>
                <small>{g.role}{g.role ? " · " : ""}{compact(g.members)} members</small>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </article>
{/if}

<style>
  .pad {
    padding: var(--s-4) var(--s-4) 0 0;
  }
  .profile {
    position: relative;
    display: grid;
    gap: var(--s-5);
    max-width: 1000px;
    margin: 0 auto;
    padding: var(--s-7) var(--s-4) var(--s-7) 0;
  }
  .back {
    position: absolute;
    top: 0;
    left: 0;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line-strong);
    background: var(--glass);
    color: var(--text-0);
    cursor: pointer;
  }
  .top {
    display: flex;
    gap: var(--s-6);
    padding: var(--s-5);
    background:
      radial-gradient(80% 120% at 0% 0%, var(--frost-glow), transparent 60%),
      var(--glass);
  }
  .figure {
    flex: none;
    width: 220px;
    height: 220px;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: radial-gradient(circle at 50% 35%, var(--glass-strong), transparent 70%);
  }
  .figure img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .initial {
    display: grid;
    place-items: center;
    height: 100%;
    font-size: 5rem;
    font-weight: 800;
    color: var(--frost);
  }
  .info {
    display: grid;
    gap: var(--s-2);
    align-content: center;
    min-width: 0;
    flex: 1;
  }
  h1 {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-size: var(--fs-2xl);
    line-height: 1.1;
    overflow-wrap: anywhere;
  }
  .check {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    flex: none;
    border-radius: 50%;
    background: var(--frost-strong);
    color: var(--frost-ink);
  }
  .check.small {
    display: inline-grid;
    width: 14px;
    height: 14px;
    margin-left: 5px;
    vertical-align: middle;
  }
  .handle {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    color: var(--text-2);
  }
  .tag {
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--glass-strong);
    color: var(--frost);
    font-size: var(--fs-xs);
    font-weight: 650;
  }
  .tag.bad {
    color: var(--danger);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: var(--fs-sm);
    color: var(--text-1);
  }
  .status i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-2);
  }
  .status.online i {
    background: var(--frost-strong);
  }
  .status.playing {
    color: var(--ok);
  }
  .status.playing i {
    background: var(--ok);
    box-shadow: 0 0 10px var(--ok);
  }
  .counts {
    display: flex;
    gap: var(--s-5);
    margin: var(--s-2) 0 0;
  }
  dt {
    font-size: 0.6875rem;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  dd {
    margin: 0;
    font-size: var(--fs-lg);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-2);
    margin-top: var(--s-2);
  }
  .actions:empty {
    display: none;
  }
  .since {
    font-size: var(--fs-xs);
  }
  .section {
    display: grid;
    gap: var(--s-3);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-size: var(--fs-lg);
  }
  .count {
    font-size: var(--fs-xs);
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--glass-hover);
    color: var(--text-1);
    font-weight: 500;
  }
  .about {
    font-size: var(--fs-sm);
    color: var(--text-1);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .people {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .people a {
    display: grid;
    justify-items: center;
    gap: 6px;
    width: 76px;
    padding: var(--s-2) 2px;
    border-radius: var(--r-md);
    color: var(--text-1);
    text-decoration: none;
    font-size: var(--fs-xs);
  }
  .people a:hover {
    background: var(--glass-hover);
    color: var(--text-0);
  }
  .people span {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .groups {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: var(--s-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .groups li {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3);
  }
  .groups img,
  .gicon {
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: var(--r-md);
    object-fit: cover;
  }
  .gicon {
    display: grid;
    place-items: center;
    background: var(--glass-strong);
    color: var(--frost);
  }
  .groups div {
    display: grid;
    min-width: 0;
  }
  .groups b {
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .groups small {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  @media (max-width: 760px) {
    .top {
      flex-direction: column;
    }
  }
</style>
