<script lang="ts">
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import CopyCommand from "$lib/components/CopyCommand.svelte";
  import GameTile from "$lib/components/GameTile.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import Skeleton from "$lib/components/Skeleton.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { DEMO_GAMES } from "$lib/demo";
  import { toast } from "$lib/state.svelte";

  const colors = ["--bg-0", "--bg-1", "--bg-2", "--frost", "--frost-strong", "--ok", "--warn", "--danger", "--text-0", "--text-1", "--text-2"];
  const icons = ["home", "games", "players", "market", "friends", "settings", "search", "play", "clock", "star", "lock", "bottle", "alert", "wifiOff", "refresh", "terminal"] as const;
  let modal = $state(false);
</script>

<div class="gallery">
  <h1>Design system</h1>

  <section>
    <h2>Color</h2>
    <div class="swatches">
      {#each colors as c (c)}<div><i style:background="var({c})"></i><code>{c}</code></div>{/each}
    </div>
  </section>

  <section>
    <h2>Type</h2>
    <h1>Heading one, 32/650</h1>
    <h2>Heading two, 18/600</h2>
    <p>Body text, 15px. Pick a game on Home and press Play.</p>
    <p class="muted">Secondary text for descriptions.</p>
    <p class="faint">Tertiary text for metadata.</p>
  </section>

  <section>
    <h2>Buttons</h2>
    <div class="line">
      <Button variant="primary" icon="play">Play</Button>
      <Button>Glass</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="danger" icon="signOut">Sign out</Button>
      <Button variant="primary" busy>Starting</Button>
      <Button disabled>Disabled</Button>
      <Button size="sm" icon="refresh">Small</Button>
      <Button size="lg" variant="primary">Large</Button>
    </div>
  </section>

  <section>
    <h2>Icons, avatars, skeletons</h2>
    <div class="line">{#each icons as name (name)}<Icon {name} />{/each}</div>
    <div class="line">
      <Avatar name="Frostbyte" size={44} status="online" />
      <Avatar name="Ari" size={36} status="playing" />
      <Avatar name="Kit" size={28} />
      <div class="skel"><Skeleton height="14px" /><Skeleton width="60%" height="12px" /></div>
    </div>
  </section>

  <section>
    <h2>Game tiles</h2>
    <div class="tiles">
      <GameTile game={DEMO_GAMES[0]} onplay={() => {}} />
      <GameTile game={DEMO_GAMES[1]} onplay={() => {}} phase="starting" />
      <GameTile game={DEMO_GAMES[9]} onplay={() => {}} />
      <GameTile game={DEMO_GAMES[2]} onplay={() => {}} phase="playing" />
      <GameTile game={DEMO_GAMES[3]} onplay={() => {}} disabled />
    </div>
  </section>

  <section>
    <h2>States</h2>
    <StateCard icon="star" title="No favorites yet">Star a game on roblox.com and it appears here.</StateCard>
    <StateCard icon="clock" tone="warn" title="Roblox asked us to slow down">Wait a minute, then retry.</StateCard>
    <StateCard icon="alert" tone="danger" title="Couldn't load favorites">Roblox sent a response Vodka didn't understand (HTTP 500).</StateCard>
    <CopyCommand label="Install Mocktail" command="flatpak install --user flathub space.bigrat.mocktail" />
  </section>

  <section>
    <h2>Toasts and modal</h2>
    <div class="line">
      <Button onclick={() => toast("Sent RIVALS to the Mocktail window that's already open.", "ok")}>Success toast</Button>
      <Button onclick={() => toast("Mocktail is not installed.", "danger")}>Error toast</Button>
      <Button onclick={() => toast("RIVALS is still starting. Give it a moment.")}>Info toast</Button>
      <Button onclick={() => (modal = true)}>Open modal</Button>
    </div>
  </section>
</div>

{#if modal}
  <Modal title="RIVALS stopped unexpectedly" onclose={() => (modal = false)}>
    <p class="muted">Vulkan device lost</p>
    {#snippet actions()}<Button onclick={() => (modal = false)}>Close</Button><Button variant="primary" icon="play">Try again</Button>{/snippet}
  </Modal>
{/if}

<style>
  .gallery,
  section {
    display: grid;
    gap: var(--s-4);
  }
  .gallery {
    gap: var(--s-6);
    max-width: 820px;
  }
  section > h2 {
    font-size: var(--fs-sm);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--frost);
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-3);
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: var(--s-3);
  }
  .swatches i {
    display: block;
    height: 48px;
    border-radius: var(--r-md);
    border: 1px solid var(--line-strong);
    margin-bottom: 4px;
  }
  .swatches code {
    font-size: var(--fs-xs);
    color: var(--text-1);
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, 150px);
    gap: var(--s-4);
  }
  .skel {
    display: grid;
    gap: 6px;
    width: 160px;
  }
</style>
