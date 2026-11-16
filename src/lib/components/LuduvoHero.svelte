<script lang="ts">
  import { luduvo, luduvoSignIn, openStudio, playLuduvo, stopLuduvo } from "$lib/luduvo.svelte";
  import Button from "./Button.svelte";
  import GameLogo from "./GameLogo.svelte";

  // Luduvo's welcome: your last game with a Play button, like Roblox's home, in Luduvo's red.
  const last = $derived(luduvo.recent.status === "ready" ? (luduvo.recent.data[0] ?? null) : null);
  const playing = $derived(luduvo.playing);
  const canPlay = $derived(!!luduvo.status?.installed && luduvo.status.supported);
  const hour = new Date().getHours();
  const greeting = hour < 5 ? "Still up" : hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
  const art = $derived(playing ? null : (last?.bannerUrl ?? last?.iconUrl ?? null));
  let broken = $state(false);

  function played(seconds: number | null): string | null {
    if (!seconds || seconds < 60) return null;
    const h = Math.floor(seconds / 3600);
    const m = Math.round((seconds % 3600) / 60);
    return h ? `${h} h ${m} min played` : `${m} min played`;
  }
  const since = (unix: number) => new Date(unix * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
</script>

<section class="hero" aria-label="Welcome to Luduvo">
  {#if art && !broken}
    <img class="backdrop" src={art} alt="" onerror={() => (broken = true)} />
  {/if}
  <div class="shade" aria-hidden="true"></div>

  <div class="copy">
    <p class="eyebrow"><GameLogo game="luduvo" size={14} />{greeting}{#if luduvo.user}, {luduvo.user.displayName}{/if}</p>

    {#if playing}
      <h1>You're in <span>{playing.title}</span></h1>
      <div class="cta">
        <span class="live"><i></i>Playing since {since(playing.startedAt)}</span>
        <Button icon="close" onclick={stopLuduvo}>Close game</Button>
      </div>
    {:else if last}
      <h1>Jump back into <span>{last.title}</span></h1>
      <div class="cta">
        <Button variant="primary" size="lg" icon="play" disabled={!canPlay} busy={luduvo.starting === last.id} onclick={() => playLuduvo(last)}>
          {luduvo.starting === last.id ? "Starting" : "Play now"}
        </Button>
        <span class="faint">
          {[last.creator && `by ${last.creator}`, played(last.playtime)].filter(Boolean).join(" · ")}
        </span>
      </div>
    {:else if luduvo.user}
      <h1>Your next round starts here</h1>
      <p class="muted">Pick a Luduvo game below and press Play. Vodka asks Luduvo for a seat and drops you straight in.</p>
    {:else if luduvo.started}
      <h1>Play Luduvo from Vodka</h1>
      <div class="cta">
        <Button variant="primary" size="lg" busy={luduvo.signingIn} onclick={luduvoSignIn}>Sign in to Luduvo</Button>
        <span class="faint">Opens luduvo.com in your browser</span>
      </div>
    {/if}
  </div>

  <div class="side">
    {#if luduvo.status?.installed}
      <Button size="sm" icon="terminal" onclick={openStudio}>Open Studio</Button>
      <span class="build">Build {luduvo.status.version ?? "?"} · updates itself</span>
    {/if}
  </div>
</section>

<style>
  .hero {
    position: relative;
    display: flex;
    align-items: center;
    min-height: 190px;
    margin-right: var(--s-4);
    padding: var(--s-5) var(--s-6);
    border-radius: var(--r-lg);
    border: 1px solid var(--line-strong);
    background:
      radial-gradient(90% 140% at 85% 30%, rgba(255, 79, 94, 0.28), transparent 60%),
      linear-gradient(120deg, #140b0e 20%, #1c0d12 70%, #2a0f16);
    box-shadow: var(--shadow-1);
    overflow: hidden;
  }
  /* The game's own wide picture, fading into the card from the right. */
  .backdrop {
    position: absolute;
    top: 0;
    right: 0;
    width: 62%;
    height: 100%;
    object-fit: cover;
    opacity: 0.75;
    mask-image: linear-gradient(90deg, transparent, #000 45%);
  }
  .shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, rgba(20, 11, 14, 0.95) 30%, rgba(20, 11, 14, 0.2) 75%, rgba(42, 15, 22, 0.35));
  }
  .copy {
    position: relative;
    display: grid;
    gap: var(--s-3);
    max-width: 60%;
  }
  .eyebrow {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--frost);
  }
  h1 {
    font-size: var(--fs-xl);
    line-height: 1.15;
  }
  h1 span {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .cta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s-3);
    font-size: var(--fs-sm);
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    height: 46px;
    padding: 0 var(--s-5);
    border-radius: var(--r-md);
    border: 1px solid rgba(116, 230, 180, 0.4);
    background: rgba(116, 230, 180, 0.1);
    color: var(--ok);
    font-weight: 650;
  }
  .live i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
    box-shadow: 0 0 10px var(--ok);
  }
  .side {
    position: absolute;
    top: var(--s-4);
    right: var(--s-4);
    display: grid;
    justify-items: end;
    gap: 6px;
  }
  .build {
    padding: 2px 8px;
    border-radius: var(--r-full);
    background: rgba(0, 0, 0, 0.45);
    font-size: 0.6875rem;
    color: var(--text-1);
  }
  @media (max-width: 900px) {
    .copy {
      max-width: 100%;
    }
    .backdrop {
      opacity: 0.35;
    }
  }
</style>
