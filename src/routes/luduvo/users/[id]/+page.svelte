<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { api } from "$lib/api";
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import LuduvoRow from "$lib/components/LuduvoRow.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { compact } from "$lib/format";
  import { joinLuduvoFriend, luduvo, startLuduvo } from "$lib/luduvo.svelte";
  import { toAppError, type Loadable, type LuduvoFriend, type LuduvoGroup, type LuduvoPlace, type LuduvoProfile } from "$lib/types";
  import { onMount } from "svelte";

  // Someone's Luduvo profile: who they are, what they're doing (with Join), what they wear, their friends, games and groups.
  onMount(startLuduvo);
  const userId = $derived(Number(page.params.id));

  let profile = $state<Loadable<LuduvoProfile>>({ status: "loading" });
  let friends = $state<Loadable<LuduvoFriend[]>>({ status: "loading" });
  let places = $state<Loadable<LuduvoPlace[]>>({ status: "loading" });
  let groups = $state<Loadable<LuduvoGroup[]>>({ status: "loading" });
  let pictureFailed = $state(false);

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
    pictureFailed = false;
    void load((v) => (profile = v), () => api.luduvoProfile(id));
    void load((v) => (friends = v), () => api.luduvoUserFriends(id));
    void load((v) => (places = v), () => api.luduvoUserPlaces(id));
    void load((v) => (groups = v), () => api.luduvoUserGroups(id));
  });

  const p = $derived(profile.status === "ready" ? profile.data : null);
  const me = $derived(luduvo.user?.id === userId);
  const isFriend = $derived(luduvo.friends.status === "ready" && luduvo.friends.data.some((f) => f.id === userId));
  const status = $derived.by(() => {
    if (!p) return null;
    if (p.state === "in_game") return { text: p.placeTitle ? `Playing ${p.placeTitle}` : "In a game", tone: "playing" };
    if (p.state === "online") return { text: "Online", tone: "online" };
    return { text: p.lastActive ? `Last seen ${ago(p.lastActive)}` : "Offline", tone: "offline" };
  });
  // Joining reuses the friends-list path, which wants a friend-shaped person.
  const asFriend = $derived<LuduvoFriend | null>(
    p ? { id: p.id, name: p.name, displayName: p.displayName, avatarUrl: p.headshotUrl, state: p.state, placeId: p.placeId, placeTitle: p.placeTitle, joinable: p.joinable } : null,
  );
  const since = $derived(p?.memberSince ? new Date(p.memberSince * 1000).toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" }) : null);
  const friendList = $derived(friends.status === "ready" ? friends.data : []);

  function ago(unix: number): string {
    const s = Math.max(0, Date.now() / 1000 - unix);
    if (s < 3600) return `${Math.max(1, Math.round(s / 60))} min ago`;
    if (s < 86400) return `${Math.round(s / 3600)} h ago`;
    return `${Math.round(s / 86400)} days ago`;
  }
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
        {#if p?.pictureUrl && !pictureFailed}
          <img src={p.pictureUrl} alt="{p.displayName}'s avatar" onerror={() => (pictureFailed = true)} />
        {:else if p}
          <span class="initial">{p.displayName.slice(0, 1).toUpperCase()}</span>
        {:else}
          <Skeleton height="100%" radius="var(--r-lg)" />
        {/if}
      </div>
      <div class="info">
        {#if p}
          <h1>{p.displayName}</h1>
          <p class="handle">
            @{p.name}
            {#if me}<span class="tag">You</span>{:else if isFriend}<span class="tag">Friend</span>{/if}
            {#if p.tier && p.tier !== "free"}<span class="tag">{p.tier}</span>{/if}
            {#each p.badges as b (b)}<span class="tag badge">{b}</span>{/each}
          </p>
          {#if p.status}<p class="mood">“{p.status}”</p>{/if}
          {#if status}<p class="status {status.tone}"><i></i>{status.text}</p>{/if}
          <dl class="counts">
            <div><dt>Friends</dt><dd>{p.friends != null ? compact(p.friends) : "?"}</dd></div>
            <div><dt>Games</dt><dd>{p.places != null ? compact(p.places) : "?"}</dd></div>
            <div><dt>Items</dt><dd>{p.items != null ? compact(p.items) : "?"}</dd></div>
          </dl>
          <div class="actions">
            {#if p.state === "in_game" && p.joinable && asFriend && !me}
              <Button variant="primary" size="lg" icon="play" disabled={!!luduvo.playing || !luduvo.user} onclick={() => asFriend && joinLuduvoFriend(asFriend)}>Join {p.displayName}</Button>
            {/if}
            {#if p.placeId}
              <Button size="lg" icon="games" onclick={() => goto(`/luduvo/games/${p.placeId}`)}>Open the game</Button>
            {/if}
          </div>
          {#if since}<p class="faint since">On Luduvo since {since}</p>{/if}
        {:else}
          <Skeleton width="60%" height="32px" />
          <Skeleton width="30%" height="14px" />
          <Skeleton width="80%" height="56px" />
        {/if}
      </div>
    </header>

    {#if p?.bio}
      <section class="section">
        <h2>About</h2>
        <p class="about selectable">{p.bio}</p>
      </section>
    {/if}

    {#if p && p.wearing.length}
      <section class="section">
        <h2>Wearing <span class="count">{p.wearing.length}</span></h2>
        <ul class="wearing">
          {#each p.wearing as w (w.id + ":" + w.variant)}
            <li class="glass">
              <span class="pic">{#if w.pictureUrl}<img src={w.pictureUrl} alt="" loading="lazy" />{:else}<Icon name="market" size={20} />{/if}</span>
              <span class="nm">{w.name}</span>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <section class="section">
      <h2>Friends {#if friendList.length}<span class="count">{friendList.length}</span>{/if}</h2>
      {#if friends.status === "loading"}
        <div class="people">{#each Array(6) as _, i (i)}<Skeleton width="72px" height="92px" radius="var(--r-md)" />{/each}</div>
      {:else if friends.status === "error"}
        <p class="faint">{friends.error.message}</p>
      {:else if friendList.length === 0}
        <p class="faint">{p?.friendsHidden ? "They keep their friends list private." : "No friends to show yet."}</p>
      {:else}
        <ul class="people">
          {#each friendList.slice(0, 60) as f (f.id)}
            <li>
              <a href="/luduvo/users/{f.id}" title="{f.displayName} (@{f.name})">
                <Avatar name={f.displayName} src={f.avatarUrl} size={56} />
                <span>{f.displayName}</span>
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <LuduvoRow
      title="Games {p ? p.displayName : 'they'} made"
      icon="games"
      row={places}
      filter={false}
      onretry={() => load((v) => (places = v), () => api.luduvoUserPlaces(userId))}
      emptyTitle="No games yet"
      emptyText="They haven't published a Luduvo game."
    />

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
              <div><b>{g.name}</b><small>{g.role}{g.role ? " · " : ""}{compact(g.members)} members</small></div>
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
    font-size: var(--fs-2xl);
    line-height: 1.1;
    overflow-wrap: anywhere;
  }
  .handle {
    display: flex;
    flex-wrap: wrap;
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
    text-transform: capitalize;
  }
  .tag.badge {
    color: var(--warn);
  }
  .mood {
    color: var(--text-1);
    font-style: italic;
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
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
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
  .wearing {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--s-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .wearing li {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: var(--s-2);
  }
  .pic {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: var(--r-sm);
    background: var(--glass-strong);
    color: var(--text-2);
    overflow: hidden;
  }
  .pic img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .nm {
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
