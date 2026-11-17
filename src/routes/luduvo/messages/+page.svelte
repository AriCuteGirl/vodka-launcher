<script lang="ts">
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { api } from "$lib/api";
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { headshot, loadChats, luduvoChat } from "$lib/luduvo-chat.svelte";
  import { buyLunes, loadBalance, luduvo, luduvoSignIn, startLuduvo } from "$lib/luduvo.svelte";
  import { toast } from "$lib/state.svelte";
  import { toAppError, type LuduvoConversation, type LuduvoFriend, type LuduvoItem, type LuduvoMessage, type LuduvoVariant } from "$lib/types";
  import { onDestroy, onMount, tick } from "svelte";

  // Luduvo Messages, laid out like luduvo.com: your chats on the left, the open one on the right.
  onMount(startLuduvo);

  const LAST_KEY = "vodka-luduvo-last-chat";
  const me = $derived(luduvo.user?.id ?? 0);

  let search = $state("");
  const shown = $derived(
    luduvoChat.list.filter((c) => !search.trim() || `${c.name} ${c.peerUsername}`.toLowerCase().includes(search.trim().toLowerCase())),
  );

  // The open chat comes from the address (?c=ID), so Back works; a new chat with a friend is a draft (?to=ID).
  const openId = $derived(Number(page.url.searchParams.get("c")) || null);
  const draftTo = $derived(Number(page.url.searchParams.get("to")) || null);
  const open = $derived(luduvoChat.list.find((c) => c.id === openId) ?? null);
  const draftFriend = $derived(
    draftTo && luduvo.friends.status === "ready" ? (luduvo.friends.data.find((f) => f.id === draftTo) ?? null) : null,
  );

  function select(id: number) {
    void goto(`/luduvo/messages?c=${id}`, { replaceState: openId != null });
  }

  // "Message" from a profile: open the chat you already have with them, if any.
  $effect(() => {
    if (!draftTo || openId || !luduvoChat.loaded) return;
    const existing = luduvoChat.list.find((c) => c.kind === "direct" && c.peerId === draftTo);
    if (existing) void goto(`/luduvo/messages?c=${existing.id}`, { replaceState: true });
  });

  // When nothing is chosen, reopen the last chat.
  $effect(() => {
    if (openId || draftTo || !luduvoChat.loaded || !luduvoChat.list.length) return;
    let last: number | null = null;
    try {
      last = Number(localStorage.getItem(LAST_KEY)) || null;
    } catch {
      // Nothing remembered.
    }
    const pick = luduvoChat.list.find((c) => c.id === last) ?? luduvoChat.list[0];
    void goto(`/luduvo/messages?c=${pick.id}`, { replaceState: true });
  });

  let messages = $state<LuduvoMessage[]>([]);
  let loading = $state(false);
  let failed = $state<string | null>(null);
  let more = $state(false);
  let loadingOlder = $state(false);
  let scroller = $state<HTMLDivElement | null>(null);
  let loadedFor = 0;

  const nearBottom = () => !scroller || scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 120;
  async function toBottom() {
    await tick();
    if (scroller) scroller.scrollTop = scroller.scrollHeight;
  }

  async function markRead() {
    const c = open;
    const last = messages.at(-1);
    if (!c || !last || document.visibilityState !== "visible") return;
    luduvoChat.list = luduvoChat.list.map((x) => (x.id === c.id ? { ...x, unread: 0, lastReadId: last.id } : x));
    if (c.lastReadId >= last.id && c.unread === 0) return;
    void api.luduvoMarkRead(c.id, last.id).catch(() => {});
  }

  async function loadChat(id: number) {
    loadedFor = id;
    loading = true;
    failed = null;
    messages = [];
    replyTo = null;
    try {
      const p = await api.luduvoMessages(id);
      if (loadedFor !== id) return;
      messages = p.messages;
      more = p.more;
      loading = false;
      await toBottom();
      void markRead();
    } catch (e) {
      if (loadedFor !== id) return;
      failed = toAppError(e).message;
      loading = false;
    }
  }

  $effect(() => {
    luduvoChat.open = openId;
    if (!openId || !luduvo.user) return;
    try {
      localStorage.setItem(LAST_KEY, String(openId));
    } catch {
      // It just won't be remembered.
    }
    void loadChat(openId);
  });
  onDestroy(() => (luduvoChat.open = null));

  async function loadOlder() {
    if (!openId || loadingOlder || !messages.length) return;
    loadingOlder = true;
    const id = openId;
    const before = scroller?.scrollHeight ?? 0;
    try {
      const p = await api.luduvoMessages(id, messages[0].id);
      if (id !== openId) return;
      const known = new Set(messages.map((m) => m.id));
      messages = [...p.messages.filter((m) => !known.has(m.id)), ...messages];
      more = p.more;
      await tick();
      // Keep the view where it was.
      if (scroller) scroller.scrollTop = scroller.scrollHeight - before;
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    loadingOlder = false;
  }

  // New messages: ask every 3 seconds while the chat is open and Vodka is in front.
  let polling = false;
  async function poll() {
    const id = openId;
    const last = messages.at(-1);
    if (!id || !last || polling || loading || document.visibilityState !== "visible") return;
    polling = true;
    try {
      const fresh = await api.luduvoMessagesSince(id, last.id);
      if (id === openId && fresh.length) {
        const stick = nearBottom();
        const known = new Set(messages.map((m) => m.id));
        messages = [...messages, ...fresh.filter((m) => !known.has(m.id))];
        if (stick) void toBottom();
        void markRead();
        void loadChats();
      }
    } catch {
      // Try again next time.
    }
    polling = false;
  }
  const timer = setInterval(poll, 3000);
  onDestroy(() => clearInterval(timer));

  // Writing and sending.
  let text = $state("");
  let sending = $state(false);
  let replyTo = $state<LuduvoMessage | null>(null);
  let input = $state<HTMLTextAreaElement | null>(null);
  let typedAt = 0;
  const MAX = 1000;

  function typed() {
    if (!openId || Date.now() - typedAt < 3000) return;
    typedAt = Date.now();
    void api.luduvoTyping(openId).catch(() => {});
  }

  async function send(e?: Event) {
    e?.preventDefault();
    const body = text.trim();
    if (!body || sending) return;
    sending = true;
    try {
      if (draftTo && !openId) {
        const id = await api.luduvoStartChat(draftTo, body);
        text = "";
        await loadChats();
        select(id);
      } else if (openId) {
        const sent = await api.luduvoSendMessage(openId, body, replyTo?.id ?? null);
        text = "";
        replyTo = null;
        if (sent) {
          messages = [...messages.filter((m) => m.id !== sent.id), sent];
        } else {
          await poll();
        }
        void toBottom();
        void loadChats();
      }
    } catch (err) {
      toast(toAppError(err).message, "danger");
    }
    sending = false;
    input?.focus();
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      void send();
    } else if (e.key === "Escape" && replyTo) {
      replyTo = null;
    }
  }

  function reply(m: LuduvoMessage) {
    replyTo = m;
    input?.focus();
  }

  // Gifts: pick a store item, confirm the price, and Luduvo sends it in this chat.
  let gifting = $state(false);
  let giftSearch = $state("");
  let giftItems = $state<LuduvoItem[] | null>(null);
  let giftError = $state<string | null>(null);
  let giftPick = $state<LuduvoItem | null>(null);
  let giftConfirm = $state(false);
  let giftSending = $state(false);
  let giftRun = 0;
  // Luduvo's rules: no free items, no limiteds.
  const giftable = (i: LuduvoItem) => i.price != null && i.price > 0 && !i.limited;
  let variants = $state<LuduvoVariant[] | null>(null);
  let variant = $state<LuduvoVariant | null>(null);
  async function pick(it: LuduvoItem) {
    giftPick = it;
    variants = null;
    variant = null;
    try {
      const list = (await api.luduvoItemVariants(it.id)).filter((v) => v.onSale);
      if (giftPick?.id !== it.id) return;
      variants = list;
      variant = list.find((v) => (v.price ?? it.price ?? 0) > 0) ?? null;
    } catch {
      if (giftPick?.id === it.id) variants = [];
    }
  }
  /** What this gift costs: the colour's own price when it has one. */
  const giftPrice = $derived(giftPick ? (variant?.price ?? giftPick.price ?? 0) : 0);
  async function findGifts() {
    const run = ++giftRun;
    giftError = null;
    try {
      const page = await api.luduvoStore(giftSearch.trim(), "", "", "relevance", 0);
      if (run === giftRun) giftItems = page.items.filter(giftable);
    } catch (e) {
      if (run === giftRun) giftError = toAppError(e).message;
    }
  }
  let giftTimer: ReturnType<typeof setTimeout> | undefined;
  function giftSearchChanged() {
    clearTimeout(giftTimer);
    giftTimer = setTimeout(findGifts, 300);
  }
  function openGifts() {
    gifting = true;
    giftPick = null;
    variants = null;
    variant = null;
    giftConfirm = false;
    if (!giftItems) void findGifts();
    void loadBalance(true);
  }
  function closeGifts() {
    gifting = false;
    giftConfirm = false;
    giftPick = null;
  }
  const giftTo = $derived(open?.kind === "direct" ? open.peerId : null);
  const tooDear = $derived(!!giftPick && luduvo.coins != null && giftPrice > luduvo.coins);
  const giftReady = $derived(!!giftPick && variants != null && giftPrice > 0);
  async function sendGift() {
    if (!giftPick || !giftReady || !giftTo || giftSending) return;
    giftSending = true;
    try {
      const balance = await api.luduvoGiftItem(giftPick.id, giftTo, giftPrice, variant?.index ?? null);
      if (balance != null) luduvo.coins = balance;
      toast(`Sent ${giftPick.name} to ${header?.name ?? "them"}.`, "ok");
      closeGifts();
      await poll();
      void toBottom();
      void loadChats();
    } catch (e) {
      toast(toAppError(e).message, "danger", 9000);
    }
    giftSending = false;
  }

  let answering = $state<number | null>(null);
  async function answerGift(m: LuduvoMessage, accept: boolean) {
    if (!m.gift || answering) return;
    answering = m.gift.id;
    try {
      await api.luduvoAnswerGift(m.gift.id, accept);
      const status = accept ? "accepted" : "rejected";
      messages = messages.map((x) => (x.id === m.id && x.gift ? { ...x, gift: { ...x.gift, status } } : x));
      toast(accept ? `${m.gift.itemName} is in your inventory.` : "Gift declined.", "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    answering = null;
  }
  const lunes = (n: number) => (n === 0 ? "Free" : `${n.toLocaleString()} Lunes`);

  // Picking a friend for a new chat.
  let picking = $state(false);
  let pickSearch = $state("");
  const friends = $derived(luduvo.friends.status === "ready" ? luduvo.friends.data : []);
  const pickable = $derived(
    friends.filter((f) => !pickSearch.trim() || `${f.displayName} ${f.name}`.toLowerCase().includes(pickSearch.trim().toLowerCase())),
  );
  function startWith(f: LuduvoFriend) {
    picking = false;
    pickSearch = "";
    const existing = luduvoChat.list.find((c) => c.kind === "direct" && c.peerId === f.id);
    if (existing) select(existing.id);
    else void goto(`/luduvo/messages?to=${f.id}`);
  }

  // Times and grouping, like the website: a date line between days, and a sender's messages a few minutes apart
  // run together under one name and picture.
  const dayOf = (s: number) => new Date(s * 1000).toDateString();
  function dayLabel(s: number): string {
    const d = new Date(s * 1000);
    const today = new Date();
    const yesterday = new Date(Date.now() - 86_400_000);
    if (d.toDateString() === today.toDateString()) return "Today";
    if (d.toDateString() === yesterday.toDateString()) return "Yesterday";
    return d.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long", year: d.getFullYear() === today.getFullYear() ? undefined : "numeric" });
  }
  const clock = (s: number) => new Date(s * 1000).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  function listTime(s: number | null): string {
    if (!s) return "";
    const ago = Date.now() / 1000 - s;
    if (ago < 60) return "now";
    if (ago < 3600) return `${Math.floor(ago / 60)}m`;
    if (ago < 86_400) return `${Math.floor(ago / 3600)}h`;
    if (ago < 7 * 86_400) return `${Math.floor(ago / 86_400)}d`;
    return new Date(s * 1000).toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
  const GROUP_SECS = 5 * 60;
  const startsGroup = (i: number) =>
    i === 0 ||
    messages[i - 1].senderId !== messages[i].senderId ||
    messages[i].sentAt - messages[i - 1].sentAt > GROUP_SECS ||
    dayOf(messages[i - 1].sentAt) !== dayOf(messages[i].sentAt) ||
    !!messages[i].replyTo;
  const endsGroup = (i: number) => i === messages.length - 1 || startsGroup(i + 1);

  function preview(c: LuduvoConversation): string {
    if (!c.lastBody) return c.lastMessageId ? "Sent something" : "No messages yet";
    return `${c.lastSenderId === me ? "You: " : ""}${c.lastBody}`;
  }
  const kindNote = (m: LuduvoMessage) => (m.kind === "gift" ? "🎁 Sent a gift" : m.kind ? "Open luduvo.com to see this message" : "");

  const header = $derived(
    open
      ? { name: open.name, sub: open.peerUsername ? `@${open.peerUsername}` : open.kind === "group" ? "Group chat" : "", peer: open.peerId }
      : draftFriend
        ? { name: draftFriend.displayName, sub: `@${draftFriend.name}`, peer: draftFriend.id }
        : null,
  );
</script>

<div class="messages">
  {#if !luduvo.user}
    {#if luduvo.started}
      <StateCard icon="lock" title="Sign in to Luduvo to see your messages">
        {#snippet actions()}<Button variant="primary" busy={luduvo.signingIn} onclick={luduvoSignIn}>Sign in to Luduvo</Button>{/snippet}
      </StateCard>
    {/if}
  {:else}
    <aside class="list glass">
      <div class="find">
        <label class="search">
          <Icon name="search" size={15} />
          <input bind:value={search} placeholder="Search conversations" spellcheck="false" autocomplete="off" aria-label="Search conversations" />
        </label>
        <button class="new" onclick={() => (picking = true)} aria-label="New message" title="New message"><Icon name="plus" size={18} /></button>
      </div>
      {#if !luduvoChat.loaded && !luduvoChat.error}
        <p class="faint pad">Loading…</p>
      {:else if luduvoChat.error && !luduvoChat.loaded}
        <p class="faint pad">{luduvoChat.error}</p>
      {:else if shown.length === 0}
        <p class="faint pad">{search ? "No chats match." : "No messages yet. Start one with the + button."}</p>
      {/if}
      <ul>
        {#each shown as c (c.id)}
          <li>
            <button class="chat" class:on={c.id === openId} class:unread={c.unread > 0 && c.id !== openId} onclick={() => select(c.id)}>
              <Avatar name={c.name} src={headshot(c.peerId)} size={40} />
              <span class="what">
                <span class="nm">{c.name}</span>
                <span class="last">{preview(c)}</span>
              </span>
              <span class="side">
                <small>{listTime(c.lastSentAt)}</small>
                {#if c.unread > 0 && c.id !== openId}<i class="dot">{c.unread > 9 ? "9+" : c.unread}</i>{/if}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    </aside>

    <section class="room glass">
      {#if header}
        <header>
          <Avatar name={header.name} src={headshot(header.peer)} size={36} />
          <span class="who">
            <b>{header.name}</b>
            {#if header.sub}<small>{header.sub}</small>{/if}
          </span>
          {#if header.peer}
            <a class="icon-btn" href="/luduvo/users/{header.peer}" title="Profile" aria-label="Open {header.name}'s profile"><Icon name="avatar" size={18} /></a>
          {/if}
        </header>

        <div class="scroll" bind:this={scroller}>
          {#if loading}
            <p class="faint center">Loading messages…</p>
          {:else if failed}
            <StateCard icon="alert" tone="danger" title="Couldn't load this chat">
              {failed}
              {#snippet actions()}<Button icon="refresh" onclick={() => openId && loadChat(openId)}>Retry</Button>{/snippet}
            </StateCard>
          {:else}
            {#if more}
              <div class="center"><Button size="sm" busy={loadingOlder} onclick={loadOlder}>Load earlier messages</Button></div>
            {:else}
              <p class="start">This is the start of your conversation. Say hello!</p>
            {/if}
            {#each messages as m, i (m.id)}
              {@const mine = m.senderId === me}
              {#if i === 0 || dayOf(messages[i - 1].sentAt) !== dayOf(m.sentAt)}
                <p class="day"><span>{dayLabel(m.sentAt)}</span></p>
              {/if}
              <div class="msg" class:mine class:first={startsGroup(i)}>
                <span class="face">
                  {#if !mine && startsGroup(i)}<Avatar name={m.senderName || "?"} src={headshot(m.senderId)} size={32} />{/if}
                </span>
                <div class="col">
                  {#if startsGroup(i) && !mine && open?.kind === "group"}<small class="from">{m.senderName}</small>{/if}
                  {#if m.replyTo}
                    <span class="quote">
                      <Icon name="back" size={11} />
                      {m.replyTo.senderName ? (m.replyTo.senderId === me ? "You" : `@${m.replyTo.senderName}`) : "Reply"}{#if m.replyTo.body}: {m.replyTo.body}{/if}
                    </span>
                  {/if}
                  <div class="line">
                    {#if m.gift}
                      {@const g = m.gift}
                      <div class="gift">
                        <span class="gpic">{#if g.pictureUrl}<img src={g.pictureUrl} alt="" />{:else}<Icon name="market" size={24} />{/if}</span>
                        <span class="gtext">
                          <small>{mine ? "You sent a gift" : `${m.senderName || "They"} sent you a gift`}</small>
                          <b>{g.itemName || "An item"}{#if g.variantName} · {g.variantName}{/if}</b>
                          {#if g.status === "pending" && !mine}
                            <span class="gbtns">
                              <Button size="sm" variant="primary" busy={answering === g.id} onclick={() => answerGift(m, true)}>Accept</Button>
                              <Button size="sm" disabled={answering === g.id} onclick={() => answerGift(m, false)}>Decline</Button>
                            </span>
                          {:else}
                            <small class="gstate {g.status}">{g.status === "accepted" ? "Accepted" : g.status === "rejected" ? "Declined" : "Waiting for them to accept"}</small>
                          {/if}
                        </span>
                      </div>
                    {:else}
                      <p class="bubble" class:note={!m.body}>{m.body || kindNote(m)}</p>
                    {/if}
                    <button class="reply" onclick={() => reply(m)} aria-label="Reply to this message" title="Reply"><Icon name="back" size={14} /></button>
                  </div>
                  {#if endsGroup(i)}<small class="time">{clock(m.sentAt)}</small>{/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>

        <form class="compose" onsubmit={send}>
          {#if replyTo}
            <div class="replying">
              <span>Replying to <b>{replyTo.senderId === me ? "yourself" : `@${replyTo.senderName}`}</b>: {replyTo.body}</span>
              <button type="button" onclick={() => (replyTo = null)} aria-label="Cancel reply"><Icon name="close" size={14} /></button>
            </div>
          {/if}
          <div class="row">
            <textarea
              bind:this={input}
              bind:value={text}
              rows="1"
              maxlength={MAX}
              placeholder="Message {header.name}"
              aria-label="Message {header.name}"
              onkeydown={keydown}
              oninput={typed}
            ></textarea>
            {#if giftTo}
              <button class="gift-btn" type="button" onclick={openGifts} aria-label="Send a gift" title="Send a gift"><Icon name="gift" size={19} /></button>
            {/if}
            <button class="send" type="submit" disabled={!text.trim() || sending} aria-label="Send"><Icon name="send" size={18} /></button>
          </div>
        </form>
      {:else}
        <div class="empty">
          <Icon name="chat" size={40} />
          <p>Pick a chat on the left, or start a new one.</p>
          <Button icon="plus" onclick={() => (picking = true)}>New message</Button>
        </div>
      {/if}
    </section>
  {/if}
</div>

{#if gifting && header}
  <Modal title={giftConfirm && giftPick ? `Gift ${giftPick.name}?` : "Send a gift"} onclose={closeGifts}>
    {#if giftConfirm && giftPick}
      <div class="gconfirm">
        <span class="gpic big">{#if variant?.pictureUrl ?? giftPick.pictureUrl}<img src={variant?.pictureUrl ?? giftPick.pictureUrl} alt="" />{:else}<Icon name="market" size={32} />{/if}</span>
        <div>
          <p><b>{giftPick.name}</b>{#if variant} · {variant.name}{/if} for <b>{header.name}</b></p>
          <p class="faint">Costs {lunes(giftPrice)}.{#if luduvo.coins != null}{" "}You have {luduvo.coins.toLocaleString()} Lunes, {Math.max(0, luduvo.coins - giftPrice).toLocaleString()} after.{/if} This can't be undone.</p>
          {#if tooDear}<p class="err">You don't have enough Lunes. <button class="link" onclick={buyLunes}>Buy Lunes</button></p>{/if}
        </div>
      </div>
    {:else}
      <p class="faint">Choose a store item to send to {header.name}.</p>
      <label class="search gsearch">
        <Icon name="search" size={15} />
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={giftSearch} oninput={giftSearchChanged} placeholder="Search store items" spellcheck="false" autocomplete="off" aria-label="Search store items" autofocus />
      </label>
      {#if giftError}
        <p class="err">{giftError}</p>
      {:else if !giftItems}
        <p class="faint">Loading store items…</p>
      {:else if giftItems.length === 0}
        <p class="faint">No giftable items found.</p>
      {:else}
        <ul class="gitems" role="listbox" aria-label="Store items">
          {#each giftItems as it (it.id)}
            <li>
              <button role="option" aria-selected={giftPick?.id === it.id} class:on={giftPick?.id === it.id} onclick={() => pick(it)}>
                <span class="gpic">{#if it.pictureUrl}<img src={it.pictureUrl} alt="" loading="lazy" />{:else}<Icon name="market" size={18} />{/if}</span>
                <span class="gname"><b>{it.name}</b><small>{lunes(it.price ?? 0)}</small></span>
                {#if giftPick?.id === it.id}<Icon name="check" size={16} />{/if}
              </button>
            </li>
          {/each}
        </ul>
        {#if giftPick && variants && variants.length > 0}
          <div class="colours" role="radiogroup" aria-label="Colour">
            {#each variants as v (v.index)}
              <button role="radio" aria-checked={variant?.index === v.index} class:on={variant?.index === v.index} onclick={() => (variant = v)}>
                <span class="sw" style:background={v.color ?? "transparent"}></span>{v.name || `Colour ${v.index}`}
                {#if v.price != null && v.price !== giftPick.price}<small>{lunes(v.price)}</small>{/if}
              </button>
            {/each}
          </div>
        {/if}
        {#if giftPick && variants != null && giftPrice === 0}<p class="err">Free items can't be gifted.</p>{/if}
      {/if}
    {/if}
    {#snippet actions()}
      {#if giftConfirm}
        <Button onclick={() => (giftConfirm = false)}>Back</Button>
        <Button variant="primary" busy={giftSending} disabled={tooDear || !giftReady} onclick={sendGift}>Yes, pay {giftPrice.toLocaleString()} Lunes</Button>
      {:else}
        <Button onclick={closeGifts}>Cancel</Button>
        <Button variant="primary" icon="gift" disabled={!giftReady} busy={!!giftPick && variants == null} onclick={() => (giftConfirm = true)}>{giftReady ? `Gift for ${lunes(giftPrice)}` : "Gift"}</Button>
      {/if}
    {/snippet}
  </Modal>
{/if}

{#if picking}
  <Modal title="New message" onclose={() => ((picking = false), (pickSearch = ""))}>
    <input class="pick-search" bind:value={pickSearch} placeholder="Search your friends" spellcheck="false" autocomplete="off" aria-label="Search your friends" />
    {#if luduvo.friends.status !== "ready"}
      <p class="faint">Loading your friends…</p>
    {:else if pickable.length === 0}
      <p class="faint">{friends.length ? "No friends match." : "Add friends on the Friends page to message them."}</p>
    {:else}
      <ul class="pick">
        {#each pickable as f (f.id)}
          <li>
            <button onclick={() => startWith(f)}>
              <Avatar name={f.displayName} src={f.avatarUrl} size={32} status={f.state === "in_game" ? "playing" : f.state === "online" ? "online" : undefined} />
              <span><b>{f.displayName}</b><small>@{f.name}</small></span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </Modal>
{/if}

<style>
  /* The page fills the panel exactly, so only the chat list and the conversation scroll. */
  :global(main:has(> .messages)) {
    overflow: hidden;
    padding-bottom: var(--s-4);
  }
  .messages {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: var(--s-3);
    height: 100%;
    min-height: 0;
    padding-right: var(--s-4);
  }
  .list,
  .room {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .find {
    display: flex;
    gap: var(--s-2);
    padding: var(--s-3);
  }
  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    height: 38px;
    border-radius: var(--r-md);
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--line);
    color: var(--text-2);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text-0);
    font-size: var(--fs-sm);
    outline: none;
    user-select: text;
  }
  .new {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border: 0;
    border-radius: var(--r-md);
    background: var(--accent, var(--frost));
    color: var(--frost-ink);
    cursor: pointer;
  }
  .pad {
    padding: 0 var(--s-4);
  }
  .list ul {
    flex: 1;
    margin: 0;
    padding: 0 var(--s-2) var(--s-2);
    list-style: none;
    overflow-y: auto;
  }
  .chat {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    width: 100%;
    padding: 10px;
    border: 0;
    border-radius: var(--r-md);
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .chat:hover {
    background: var(--glass-hover);
  }
  .chat.on {
    background: var(--glass-strong);
  }
  .what {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  .nm,
  .last {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nm {
    font-weight: 600;
    font-size: var(--fs-sm);
  }
  .last {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .chat.unread .nm,
  .chat.unread .last {
    color: var(--text-0);
    font-weight: 700;
  }
  .side {
    display: grid;
    justify-items: end;
    gap: 4px;
    color: var(--text-2);
    font-size: 0.6875rem;
  }
  .dot {
    min-width: 18px;
    padding: 0 5px;
    border-radius: var(--r-full);
    background: var(--danger);
    color: #fff;
    font-style: normal;
    font-weight: 700;
    text-align: center;
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4);
    border-bottom: 1px solid var(--line);
  }
  .who {
    flex: 1;
    display: grid;
    min-width: 0;
  }
  .who small {
    color: var(--text-2);
    font-size: var(--fs-xs);
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    color: var(--text-1);
  }
  .icon-btn:hover {
    background: var(--glass-hover);
    color: var(--text-0);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--s-4);
    display: flex;
    flex-direction: column;
  }
  .center {
    display: flex;
    justify-content: center;
    margin: var(--s-2) 0;
  }
  .start,
  .day span {
    align-self: center;
    padding: 4px 14px;
    border-radius: var(--r-full);
    background: var(--glass);
    color: var(--text-2);
    font-size: var(--fs-xs);
  }
  .start {
    margin: var(--s-2) 0 var(--s-4);
  }
  .day {
    display: flex;
    justify-content: center;
    margin: var(--s-4) 0 var(--s-2);
  }
  .msg {
    display: flex;
    gap: var(--s-2);
    margin-top: 3px;
  }
  .msg.first {
    margin-top: var(--s-3);
  }
  .msg.mine {
    flex-direction: row-reverse;
  }
  .face {
    width: 32px;
    flex: none;
  }
  .mine .face {
    display: none;
  }
  .col {
    display: grid;
    justify-items: start;
    gap: 3px;
    max-width: min(560px, 75%);
  }
  .mine .col {
    justify-items: end;
  }
  .from {
    color: var(--text-2);
    font-size: var(--fs-xs);
  }
  .quote {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 100%;
    padding: 2px 8px;
    border-left: 2px solid var(--line-strong);
    color: var(--text-2);
    font-size: var(--fs-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .mine .line {
    flex-direction: row-reverse;
  }
  .bubble {
    margin: 0;
    padding: 8px 13px;
    border-radius: 16px;
    background: var(--glass-strong);
    color: var(--text-0);
    font-size: var(--fs-sm);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .mine .bubble {
    background: var(--accent, var(--frost));
    color: var(--frost-ink);
  }
  .bubble.note {
    font-style: italic;
    color: var(--text-1);
  }
  .reply {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 50%;
    background: none;
    color: var(--text-2);
    cursor: pointer;
    opacity: 0;
  }
  .msg:hover .reply,
  .reply:focus-visible {
    opacity: 1;
  }
  .reply:hover {
    background: var(--glass-hover);
    color: var(--text-0);
  }
  .time {
    color: var(--text-2);
    font-size: 0.6875rem;
  }
  .compose {
    display: grid;
    gap: 6px;
    padding: var(--s-3);
    border-top: 1px solid var(--line);
  }
  .replying {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: 4px 10px;
    border-radius: var(--r-sm);
    background: var(--glass);
    color: var(--text-1);
    font-size: var(--fs-xs);
  }
  .replying span {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .replying button {
    border: 0;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .row {
    display: flex;
    align-items: flex-end;
    gap: var(--s-2);
  }
  textarea {
    flex: 1;
    min-height: 42px;
    max-height: 140px;
    padding: 11px 14px;
    border-radius: var(--r-md);
    border: 1px solid var(--line);
    background: rgba(0, 0, 0, 0.25);
    color: var(--text-0);
    font: inherit;
    font-size: var(--fs-sm);
    resize: none;
    field-sizing: content;
    user-select: text;
  }
  textarea:focus {
    outline: none;
    border-color: var(--frost);
  }
  .send {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    border: 0;
    border-radius: var(--r-md);
    background: var(--accent, var(--frost));
    color: var(--frost-ink);
    cursor: pointer;
  }
  .send:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .empty {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: var(--s-3);
    color: var(--text-2);
  }
  .gift-btn {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    border: 0;
    border-radius: var(--r-md);
    background: none;
    color: var(--text-1);
    cursor: pointer;
  }
  .gift-btn:hover {
    background: var(--glass-hover);
    color: var(--frost);
  }
  .gift {
    display: flex;
    gap: var(--s-3);
    padding: var(--s-3);
    border-radius: 16px;
    border: 1px solid var(--line-strong);
    background: var(--glass-strong);
    min-width: 240px;
  }
  .gpic {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    flex: none;
    border-radius: var(--r-md);
    background: var(--glass);
    color: var(--text-2);
    overflow: hidden;
  }
  .gpic.big {
    width: 80px;
    height: 80px;
  }
  .gpic img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .gtext {
    display: grid;
    gap: 3px;
    align-content: center;
  }
  .gtext small {
    color: var(--text-2);
    font-size: var(--fs-xs);
  }
  .gstate.accepted {
    color: var(--ok);
  }
  .gbtns {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }
  .gsearch {
    margin: var(--s-2) 0 var(--s-3);
  }
  .gitems {
    display: grid;
    gap: 2px;
    max-height: 300px;
    margin: 0;
    padding: 4px;
    list-style: none;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-md);
  }
  .gitems button {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    width: 100%;
    padding: 6px 8px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .gitems button:hover {
    background: var(--glass-hover);
  }
  .gitems button.on {
    background: var(--glass-strong);
    color: var(--frost);
  }
  .gitems .gpic {
    width: 40px;
    height: 40px;
  }
  .gname {
    flex: 1;
    display: grid;
    min-width: 0;
  }
  .gname small {
    color: var(--text-2);
    font-size: var(--fs-xs);
  }
  .colours {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: var(--s-3);
  }
  .colours button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px 4px 6px;
    border-radius: var(--r-full);
    border: 1px solid var(--line);
    background: var(--glass);
    color: var(--text-1);
    font-size: var(--fs-xs);
    cursor: pointer;
  }
  .colours button.on {
    border-color: var(--frost);
    color: var(--text-0);
  }
  .colours small {
    color: var(--text-2);
  }
  .sw {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.25);
  }
  .gconfirm {
    display: flex;
    gap: var(--s-4);
    align-items: center;
  }
  .gconfirm p {
    margin: 0 0 4px;
  }
  .err {
    color: var(--danger);
    font-size: var(--fs-sm);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--frost);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .pick-search {
    width: 100%;
    height: 36px;
    margin-bottom: var(--s-3);
    padding: 0 var(--s-3);
    border-radius: var(--r-sm);
    border: 1px solid var(--line-strong);
    background: rgba(0, 0, 0, 0.25);
    color: var(--text-0);
    user-select: text;
  }
  .pick {
    display: grid;
    gap: 2px;
    max-height: 360px;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow-y: auto;
  }
  .pick button {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    width: 100%;
    padding: 8px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .pick button:hover {
    background: var(--glass-hover);
  }
  .pick span {
    display: grid;
  }
  .pick small {
    color: var(--text-2);
    font-size: var(--fs-xs);
  }
  @media (max-width: 860px) {
    :global(main:has(> .messages)) {
      overflow-y: auto;
    }
    .messages {
      grid-template-columns: 1fr;
      height: auto;
    }
    .room {
      min-height: 480px;
    }
  }
</style>
