<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import Skeleton from "./Skeleton.svelte";

  // A game's own page, shared by Roblox and Luduvo: the banner fades in behind the title and Play button,
  // then pictures, then whatever sections the game adds (servers, about) as you scroll.
  let {
    title,
    banner = null,
    icon = null,
    creator = null,
    verified = false,
    subtitle = null,
    media = [],
    loading = false,
    actions,
    pills,
    children,
  }: {
    title: string;
    banner?: string | null;
    icon?: string | null;
    creator?: string | null;
    verified?: boolean;
    /** A short line under the title: genre, age rating. */
    subtitle?: string | null;
    /** Wide pictures for the viewer. */
    media?: string[];
    loading?: boolean;
    actions: Snippet;
    pills?: Snippet;
    children?: Snippet;
  } = $props();

  let shown = $state(0);
  let iconFailed = $state(false);
  const backdrop = $derived(banner ?? media[0] ?? icon);
  const current = $derived(media[Math.min(shown, media.length - 1)] ?? null);
</script>

<div class="wrap">
  {#if backdrop}
    <div class="backdrop" aria-hidden="true"><img src={backdrop} alt="" class:soft={!banner && !media.length} /></div>
  {/if}
<article class="page">

  <button class="back" onclick={() => history.back()} aria-label="Back"><Icon name="back" size={18} /></button>

  <header class="top">
    <div class="icon">
      {#if icon && !iconFailed}
        <img src={icon} alt="" onerror={() => (iconFailed = true)} />
      {:else}
        <span>{title.replace(/[^\p{L}\p{N}]/gu, "").slice(0, 1).toUpperCase() || "?"}</span>
      {/if}
    </div>
    <div class="heading">
      <h1>{title}</h1>
      {#if subtitle}<p class="sub">{subtitle}</p>{/if}
      {#if creator}
        <p class="by">{creator}{#if verified}<span class="check" title="Verified creator"><Icon name="check" size={11} /></span>{/if}</p>
      {:else if loading}
        <Skeleton width="140px" height="14px" />
      {/if}
      <div class="actions">{@render actions()}</div>
    </div>
  </header>

  {#if pills}<div class="pills">{@render pills()}</div>{/if}

  {#if current}
    <section class="media" aria-label="Pictures">
      <img class="shot" src={current} alt="{title}, picture {shown + 1} of {media.length}" />
      {#if media.length > 1}
        <button class="nav prev" onclick={() => (shown = (shown - 1 + media.length) % media.length)} aria-label="Previous picture"><Icon name="back" size={18} /></button>
        <button class="nav next" onclick={() => (shown = (shown + 1) % media.length)} aria-label="Next picture"><Icon name="forward" size={18} /></button>
        <div class="dots">
          {#each media as _, i (i)}<button class:on={i === shown} onclick={() => (shown = i)} aria-label="Picture {i + 1}"></button>{/each}
        </div>
      {/if}
    </section>
  {/if}

  {#if children}{@render children()}{/if}
</article>
</div>

<style>
  .page {
    position: relative;
    display: grid;
    gap: var(--s-5);
    max-width: 980px;
    margin: 0 auto;
    padding: var(--s-6) var(--s-4) var(--s-7) 0;
  }
  /* The banner sits behind the top of the page, across the whole panel, and fades into it. */
  .wrap {
    position: relative;
    overflow-x: clip;
  }
  .backdrop {
    position: absolute;
    top: calc(var(--s-4) * -1);
    left: 0;
    right: 0;
    height: 360px;
    overflow: hidden;
    pointer-events: none;
    mask-image: linear-gradient(180deg, #000 10%, transparent);
  }
  .backdrop img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0.4;
  }
  .backdrop img.soft {
    filter: blur(30px) saturate(1.4);
    transform: scale(1.2);
  }
  .back {
    position: absolute;
    top: 0;
    left: 0;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line-strong);
    background: rgba(8, 12, 17, 0.6);
    color: var(--text-0);
    cursor: pointer;
  }
  .back:hover {
    background: var(--glass-strong);
  }
  .top {
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: var(--s-5);
    padding-top: var(--s-6);
  }
  .icon {
    flex: none;
    width: 176px;
    height: 176px;
    border-radius: var(--r-xl);
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background: var(--glass-strong);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.55);
  }
  .icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .icon span {
    display: grid;
    place-items: center;
    height: 100%;
    font-size: 4rem;
    font-weight: 800;
    color: var(--frost);
  }
  .heading {
    display: grid;
    gap: 6px;
    min-width: 0;
  }
  h1 {
    font-size: var(--fs-2xl);
    line-height: 1.1;
    text-shadow: 0 2px 16px rgba(0, 0, 0, 0.5);
    overflow-wrap: anywhere;
  }
  .sub {
    font-size: var(--fs-sm);
    color: var(--text-1);
  }
  .by {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .check {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--frost-strong);
    color: var(--frost-ink);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-2);
    margin-top: var(--s-2);
  }
  .actions :global(.btn.lg.primary) {
    min-width: 220px;
  }
  .pills {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-2);
  }
  .pills :global(.pill) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 14px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    background: var(--glass);
    color: var(--text-0);
    font-size: var(--fs-sm);
    font-weight: 600;
    cursor: pointer;
    text-decoration: none;
  }
  .pills :global(.pill small) {
    color: var(--text-2);
    font-weight: 500;
  }
  .pills :global(.pill:hover:not(:disabled):not(.static)) {
    background: var(--glass-hover);
    border-color: var(--line-strong);
  }
  .pills :global(.pill:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .pills :global(.pill.static) {
    cursor: default;
  }
  .pills :global(.pill.on) {
    border-color: var(--frost);
    color: var(--frost);
    background: var(--glass-strong);
  }
  .pills :global(.pill.liked) {
    border-color: rgba(116, 230, 180, 0.45);
    background: rgba(116, 230, 180, 0.12);
    color: var(--ok);
  }
  .pills :global(.pill.disliked) {
    border-color: rgba(255, 155, 155, 0.45);
    background: rgba(255, 155, 155, 0.12);
    color: var(--danger);
  }
  .pills :global(.pill .down) {
    transform: rotate(180deg);
  }
  .media {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: var(--r-lg);
    overflow: hidden;
    border: 1px solid var(--line);
    background: #000;
  }
  .shot {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .nav {
    position: absolute;
    top: 50%;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    border: 1px solid var(--line-strong);
    background: rgba(8, 12, 17, 0.7);
    color: var(--text-0);
    cursor: pointer;
    transform: translateY(-50%);
  }
  .prev {
    left: var(--s-3);
  }
  .next {
    right: var(--s-3);
  }
  .dots {
    position: absolute;
    bottom: var(--s-3);
    left: 50%;
    display: flex;
    gap: 6px;
    transform: translateX(-50%);
  }
  .dots button {
    width: 8px;
    height: 8px;
    padding: 0;
    border: 0;
    border-radius: var(--r-full);
    background: rgba(255, 255, 255, 0.45);
    cursor: pointer;
    transition: width var(--fast) var(--ease);
  }
  .dots button.on {
    width: 24px;
    background: #fff;
  }
  /* Sections the game adds below: About, Servers. */
  .page :global(.section) {
    position: relative;
    display: grid;
    gap: var(--s-3);
  }
  .page :global(.section > h2) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: var(--fs-lg);
  }
  @media (max-width: 760px) {
    .top {
      flex-direction: column;
      align-items: flex-start;
    }
    .icon {
      width: 120px;
      height: 120px;
    }
  }
</style>
