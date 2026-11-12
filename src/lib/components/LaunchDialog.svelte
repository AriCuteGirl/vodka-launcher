<script lang="ts">
  import { api } from "$lib/api";
  import { app, launchMany } from "$lib/state.svelte";
  import { toAppError, type Loadable, type ServerLists, type Target } from "$lib/types";
  import Avatar from "./Avatar.svelte";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import Skeleton from "./Skeleton.svelte";

  const { game, target: fixedTarget, targetLabel } = app.launchDialog!;

  // Start from the accounts ticked on Players, or just the active one.
  let chosen = $state<number[]>(
    (app.batch.length ? app.batch : app.account ? [app.account.id] : []).filter((id) => !app.accounts.find((a) => a.id === id)?.inVodkaGame),
  );
  /** "auto" lets Roblox choose; "fewest" is the emptiest public server; otherwise "s:<id>" or "p:<code>". */
  let choice = $state("auto");
  let servers = $state<Loadable<ServerLists>>({ status: "loading" });

  async function loadServers() {
    servers = { status: "loading" };
    try {
      servers = { status: "ready", data: await api.listServers(game.placeId) };
    } catch (e) {
      servers = { status: "error", error: toAppError(e) };
    }
  }
  if (!fixedTarget) void loadServers();

  const lists = $derived(servers.status === "ready" ? servers.data : null);
  const emptiest = $derived(lists?.public[0] ?? null);

  function toggle(id: number) {
    chosen = chosen.includes(id) ? chosen.filter((x) => x !== id) : [...chosen, id];
  }

  function target(): Target | null {
    if (fixedTarget) return fixedTarget;
    if (choice === "fewest" && emptiest) return { kind: "server", placeId: game.placeId, gameId: emptiest.id };
    if (choice.startsWith("s:")) return { kind: "server", placeId: game.placeId, gameId: choice.slice(2) };
    if (choice.startsWith("p:")) return { kind: "private", accessCode: choice.slice(2) };
    return null;
  }

  function start() {
    const ids = [...chosen];
    const where = target();
    app.launchDialog = null;
    void launchMany(game, ids, where);
  }

  const close = () => (app.launchDialog = null);
</script>

<Modal title={game.name} onclose={close}>
  <h3>Accounts</h3>
  <ul class="accounts">
    {#each app.accounts as a (a.id)}
      <li>
        <label class:busy={a.inVodkaGame}>
          <input type="checkbox" checked={chosen.includes(a.id)} disabled={a.inVodkaGame} onchange={() => toggle(a.id)} />
          <Avatar name={a.displayName} src={a.avatarUrl} size={26} />
          <span>{a.displayName}</span>
          {#if a.inVodkaGame}<em>In a game</em>{/if}
        </label>
      </li>
    {/each}
  </ul>

  <h3>Server</h3>
  {#if fixedTarget}
    <div class="option fixed"><Icon name="friends" size={18} /><span>{targetLabel ?? "A chosen server"}</span></div>
  {:else}
    <div class="servers" role="radiogroup" aria-label="Server">
      <label class="option">
        <input type="radio" bind:group={choice} value="auto" />
        <span><b>Let Roblox choose</b><small>The usual matchmaking. Accounts may land in different servers.</small></span>
      </label>
      <label class="option" class:off={!emptiest}>
        <input type="radio" bind:group={choice} value="fewest" disabled={!emptiest} />
        <span>
          <b>Fewest players</b>
          <small>
            {#if emptiest}Right now: {emptiest.playing} of {emptiest.maxPlayers} players. All chosen accounts join it together.
            {:else if servers.status === "loading"}Looking for servers…
            {:else}No public server with room was found.{/if}
          </small>
        </span>
      </label>

      {#if servers.status === "loading"}
        <Skeleton height="36px" /><Skeleton height="36px" />
      {:else if servers.status === "error"}
        <p class="err" role="alert">{servers.error.message} <button class="link" onclick={loadServers}>Retry</button></p>
      {:else if lists}
        {#if lists.private.length}
          <h4>Private servers</h4>
          {#each lists.private as p (p.accessCode)}
            <label class="option slim">
              <input type="radio" bind:group={choice} value={"p:" + p.accessCode} />
              <span><b>{p.name}</b><small>{p.owner ? `${p.owner}'s · ` : ""}{p.playing}{p.maxPlayers ? ` of ${p.maxPlayers}` : ""} players</small></span>
            </label>
          {/each}
        {:else}
          <p class="faint note">{lists.privateError ? `Private servers couldn't be checked: ${lists.privateError}` : "No private servers you can join for this game."}</p>
        {/if}
        {#if lists.public.length}
          <h4>Public servers <button class="link" onclick={loadServers}><Icon name="refresh" size={13} />Refresh</button></h4>
          <div class="list">
            {#each lists.public.slice(0, 30) as s (s.id)}
              <label class="option slim">
                <input type="radio" bind:group={choice} value={"s:" + s.id} />
                <span class="row"><b>{s.playing} of {s.maxPlayers} players</b><small>{s.ping != null ? `${s.ping} ms` : ""}</small></span>
              </label>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  {/if}

  {#snippet actions()}
    <Button onclick={close}>Cancel</Button>
    <Button variant="primary" icon="play" disabled={chosen.length === 0 || !app.mocktail?.installed} onclick={start}>
      {chosen.length > 1 ? `Start for ${chosen.length} accounts` : "Start"}
    </Button>
  {/snippet}
</Modal>

<style>
  h3 {
    margin: var(--s-3) 0 var(--s-2);
    font-size: var(--fs-xs);
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-1);
  }
  h4 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: var(--s-3) 0 2px;
    font-size: var(--fs-sm);
    font-weight: 600;
  }
  .accounts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .accounts label {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: 5px 12px 5px 8px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    background: var(--glass);
    font-size: var(--fs-sm);
    cursor: pointer;
  }
  .accounts label:has(input:checked) {
    border-color: rgba(164, 222, 255, 0.55);
    background: var(--glass-strong);
  }
  .accounts label.busy {
    opacity: 0.55;
    cursor: not-allowed;
  }
  em {
    font-style: normal;
    font-size: 0.6875rem;
    color: var(--text-2);
  }
  input[type="checkbox"],
  input[type="radio"] {
    accent-color: var(--frost-strong);
    width: 16px;
    height: 16px;
    margin: 0;
    flex: none;
  }
  .servers {
    display: grid;
    gap: 6px;
  }
  .option {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: 10px var(--s-3);
    border-radius: var(--r-md);
    border: 1px solid var(--line);
    background: rgba(0, 0, 0, 0.25);
    cursor: pointer;
  }
  .option:has(input:checked) {
    border-color: rgba(164, 222, 255, 0.55);
    background: var(--glass-hover);
  }
  .option.off {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .option.fixed {
    cursor: default;
    color: var(--frost);
  }
  .option.fixed span {
    color: var(--text-0);
    font-weight: 600;
  }
  .option span {
    display: grid;
    flex: 1;
    min-width: 0;
    line-height: 1.3;
  }
  .option .row {
    display: flex;
    justify-content: space-between;
    gap: var(--s-3);
  }
  .option.slim {
    padding: 7px var(--s-3);
  }
  b {
    font-size: var(--fs-sm);
    font-weight: 600;
  }
  small {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .list {
    display: grid;
    gap: 4px;
    max-height: 190px;
    overflow-y: auto;
    padding-right: 4px;
  }
  .note,
  .err {
    font-size: var(--fs-xs);
    margin: 4px 0 0;
  }
  .err {
    color: var(--danger);
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--frost);
    font-size: var(--fs-xs);
    font-weight: 500;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
</style>
