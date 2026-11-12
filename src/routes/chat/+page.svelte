<script lang="ts">
  import { api } from "$lib/api";
  import { acceptInvite, chat, markSeen, sendInvite, sendText, syncChat, unreadFor } from "$lib/chat.svelte";
  import Avatar from "$lib/components/Avatar.svelte";
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import StateCard from "$lib/components/StateCard.svelte";
  import { app, myInstance, signIn, toast } from "$lib/state.svelte";
  import { toAppError, type ChatContact } from "$lib/types";
  import { onMount, tick } from "svelte";

  let challenge = $state<string | null>(null);
  let verifying = $state(false);
  let verifyError = $state<string | null>(null);
  let code = $state("");
  let adding = $state(false);
  let draft = $state("");
  let sending = $state(false);
  let menu = $state(false);
  let reporting = $state<ChatContact | null>(null);
  let reason = $state("");
  let confirmDelete = $state(false);
  let thread = $state<HTMLElement>();

  onMount(() => {
    chat.visible = true;
    void syncChat();
    return () => (chat.visible = false);
  });

  const me = $derived(chat.me?.robloxId ?? -1);
  const peer = $derived(chat.contacts.find((c) => c.robloxId === chat.open) ?? null);
  const messages = $derived(peer ? chat.messages.filter((m) => m.sender === peer.robloxId || m.recipient === peer.robloxId) : []);
  const sorted = $derived(
    [...chat.contacts].sort((a, b) => Number(b.online) - Number(a.online) || a.displayName.localeCompare(b.displayName)),
  );

  // Keep the newest message in view and count it as read.
  $effect(() => {
    void messages.length;
    if (!peer) return;
    markSeen(peer.robloxId);
    void tick().then(() => thread?.scrollTo({ top: thread.scrollHeight }));
  });

  async function begin() {
    verifyError = null;
    try {
      challenge = (await api.chatStartVerify()).challenge;
    } catch (e) {
      verifyError = toAppError(e).message;
    }
  }

  async function finish() {
    verifying = true;
    verifyError = null;
    try {
      await api.chatFinishVerify();
      challenge = null;
      chat.status = "on";
      await syncChat();
      toast("Chat is on. You can remove the phrase from your Roblox profile now.", "ok");
    } catch (e) {
      verifyError = toAppError(e).message;
    }
    verifying = false;
  }

  async function copy(text: string, what: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast(`${what} copied.`, "ok");
    } catch {
      toast("Couldn't reach the clipboard. Select the text and copy it by hand.", "danger");
    }
  }

  async function add() {
    adding = true;
    try {
      const { contact } = await api.chatAddContact(code);
      code = "";
      await syncChat();
      chat.open = contact.robloxId;
      toast(`${contact.displayName} added.`, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
    adding = false;
  }

  async function send() {
    if (!peer || !draft.trim() || sending) return;
    sending = true;
    const text = draft;
    draft = "";
    if (!(await sendText(peer.robloxId, text))) draft = text;
    sending = false;
  }

  async function act(run: () => Promise<unknown>, done: string) {
    menu = false;
    try {
      await run();
      chat.open = null;
      await syncChat();
      toast(done, "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
  }

  async function turnOff(remove: boolean) {
    confirmDelete = false;
    try {
      await (remove ? api.chatDeleteAccount() : api.chatSignOut());
      chat.status = "off";
      chat.me = null;
      chat.contacts = [];
      chat.messages = [];
      chat.lastId = 0;
      toast(remove ? "Your chat account and its messages were deleted." : "Chat is off on this computer.", "ok");
    } catch (e) {
      toast(toAppError(e).message, "danger");
    }
  }

  const time = (iso: string) => new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
</script>

<svelte:window onclick={() => (menu = false)} />

<div class="chat">
  {#if !app.account}
    <StateCard icon="lock" title="Sign in to use chat">
      Chat belongs to a Roblox account, so pick one first.
      {#snippet actions()}<Button variant="primary" busy={app.signingIn} onclick={signIn}>Sign in</Button>{/snippet}
    </StateCard>
  {:else if chat.status === "unknown"}
    <p class="faint">Checking chat…</p>
  {:else if chat.status === "error"}
    <StateCard icon="alert" tone="danger" title="Chat isn't reachable">
      {chat.error}
      {#snippet actions()}<Button icon="refresh" onclick={syncChat}>Retry</Button>{/snippet}
    </StateCard>
  {:else if chat.status === "off"}
    <section class="intro glass">
      <span class="badge"><Icon name="chat" size={28} /></span>
      <h1>Vodka chat</h1>
      <p class="muted">Message other Vodka users and invite them to your game. It's Vodka's own chat, separate from Roblox chat.</p>
      <ul>
        <li><Icon name="check" size={16} />Only people you add with a Vodka code can message you. You can block or report anyone.</li>
        <li><Icon name="lock" size={16} />Your Roblox session never leaves this computer. The chat server gets your Roblox user id, your names, and what you send.</li>
        <li><Icon name="alert" size={16} />Roblox doesn't filter or moderate this chat. Messages are deleted after 14 days.</li>
      </ul>
      {#if !challenge}
        <Button variant="primary" size="lg" onclick={begin}>Turn on chat for {app.account.displayName}</Button>
      {:else}
        <div class="verify">
          <h2>Prove this is your account</h2>
          <ol>
            <li>Copy this phrase:</li>
          </ol>
          <div class="phrase">
            <code class="selectable">{challenge}</code>
            <Button size="sm" icon="copy" onclick={() => copy(challenge!, "Phrase")}>Copy</Button>
          </div>
          <ol start="2">
            <li>On roblox.com, open your profile, edit <b>About</b>, paste the phrase anywhere in it, and save.</li>
            <li>Come back and press Verify. Afterwards you can remove the phrase again.</li>
          </ol>
          <div class="row">
            <Button variant="primary" busy={verifying} onclick={finish}>Verify</Button>
            <Button variant="ghost" onclick={() => (challenge = null)}>Cancel</Button>
            <span class="faint">The phrase works for 15 minutes.</span>
          </div>
        </div>
      {/if}
      {#if verifyError}<p class="err" role="alert">{verifyError}</p>{/if}
    </section>
  {:else}
    <div class="panes">
      <aside class="glass">
        <div class="mine">
          <span class="label">Your Vodka code</span>
          <button class="codebtn" onclick={() => chat.me && copy(chat.me.code, "Your code")} title="Copy your code">
            <code>{chat.me?.code ?? "…"}</code><Icon name="copy" size={14} />
          </button>
          <span class="faint tiny">Give it to someone so they can add you.</span>
        </div>
        <form
          class="add"
          onsubmit={(e) => {
            e.preventDefault();
            void add();
          }}
        >
          <input bind:value={code} placeholder="Add by code: VDK-…" spellcheck="false" autocomplete="off" aria-label="Someone's Vodka code" maxlength="20" />
          <Button type="submit" size="sm" icon="plus" busy={adding} disabled={!code.trim()} aria-label="Add contact" />
        </form>
        {#if sorted.length === 0}
          <p class="faint empty">No contacts yet. Swap codes with a friend who uses Vodka.</p>
        {:else}
          <ul class="contacts">
            {#each sorted as c (c.robloxId)}
              {@const unread = unreadFor(c.robloxId)}
              <li>
                <button class:selected={chat.open === c.robloxId} onclick={() => (chat.open = c.robloxId)}>
                  <Avatar name={c.displayName} src={chat.pictures[String(c.robloxId)]} size={32} status={c.online ? "online" : undefined} />
                  <span class="who"><b>{c.displayName}</b><small>{c.online ? "On Vodka now" : "Away"}</small></span>
                  {#if unread}<i>{unread}</i>{/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        <div class="foot">
          <button class="link" onclick={() => turnOff(false)}>Turn off on this PC</button>
          <button class="link danger" onclick={() => (confirmDelete = true)}>Delete chat account</button>
        </div>
      </aside>

      <section class="glass thread">
        {#if !peer}
          <div class="pick"><Icon name="chat" size={32} /><p class="muted">Pick someone on the left to start talking.</p></div>
        {:else}
          <header>
            <Avatar name={peer.displayName} src={chat.pictures[String(peer.robloxId)]} size={34} status={peer.online ? "online" : undefined} />
            <div class="who"><b>{peer.displayName}</b><small>@{peer.name}</small></div>
            <Button size="sm" icon="games" disabled={!myInstance()} title={myInstance() ? "" : "Start a game first"} onclick={() => sendInvite(peer.robloxId)}>
              Invite to my game
            </Button>
            <div class="menuwrap">
              <button
                class="iconbtn"
                aria-label="More"
                aria-haspopup="menu"
                aria-expanded={menu}
                onclick={(e) => {
                  e.stopPropagation();
                  menu = !menu;
                }}><Icon name="more" /></button
              >
              {#if menu}
                <div class="menu" role="menu">
                  <button role="menuitem" onclick={() => act(() => api.chatRemoveContact(peer.robloxId), `${peer.displayName} removed.`)}>Remove contact</button>
                  <button role="menuitem" onclick={() => act(() => api.chatBlock(peer.robloxId, true), `${peer.displayName} blocked.`)}>Block</button>
                  <button
                    role="menuitem"
                    onclick={() => {
                      menu = false;
                      reporting = peer;
                    }}>Report…</button
                  >
                </div>
              {/if}
            </div>
          </header>
          <div class="messages" bind:this={thread} aria-live="polite">
            {#if messages.length === 0}
              <p class="faint hello">No messages yet. Say hi.</p>
            {/if}
            {#each messages as m (m.id)}
              <div class="msg" class:own={m.sender === me}>
                {#if m.kind === "invite" && m.payload}
                  <div class="invite">
                    {#if m.payload.game.iconUrl}<img src={m.payload.game.iconUrl} alt="" />{/if}
                    <div>
                      <small>{m.sender === me ? "You invited them to" : "Invited you to"}</small>
                      <b>{m.payload.game.name}</b>
                      <small>{m.payload.gameId ? "Same server" : "Any server"}</small>
                    </div>
                    {#if m.sender !== me}
                      <Button size="sm" variant="primary" icon="play" disabled={!app.mocktail?.installed} onclick={() => acceptInvite(m.payload!)}>Join</Button>
                    {/if}
                  </div>
                {:else}
                  <p class="selectable">{m.body}</p>
                {/if}
                <time datetime={m.createdAt}>{time(m.createdAt)}</time>
              </div>
            {/each}
          </div>
          <form
            class="composer"
            onsubmit={(e) => {
              e.preventDefault();
              void send();
            }}
          >
            <input bind:value={draft} placeholder="Message {peer.displayName}" maxlength="1000" autocomplete="off" aria-label="Message" />
            <Button type="submit" variant="primary" icon="send" disabled={!draft.trim() || sending} aria-label="Send" />
          </form>
          {#if chat.error}<p class="err small" role="status">Reconnecting… {chat.error}</p>{/if}
        {/if}
      </section>
    </div>
  {/if}
</div>

{#if reporting}
  {@const who = reporting}
  <Modal title="Report {who.displayName}" onclose={() => (reporting = null)}>
    <p class="muted">Tell Vodka's maintainer what happened. Reports are read by a person, not acted on automatically. Block them too if you don't want more messages.</p>
    <textarea bind:value={reason} rows="4" maxlength="500" placeholder="What happened?"></textarea>
    {#snippet actions()}
      <Button onclick={() => (reporting = null)}>Cancel</Button>
      <Button
        variant="danger"
        disabled={!reason.trim()}
        onclick={async () => {
          const text = reason;
          reporting = null;
          reason = "";
          try {
            await api.chatReport(who.robloxId, text);
            toast("Report sent.", "ok");
          } catch (e) {
            toast(toAppError(e).message, "danger");
          }
        }}>Send report</Button
      >
    {/snippet}
  </Modal>
{/if}

{#if confirmDelete}
  <Modal title="Delete your chat account?" onclose={() => (confirmDelete = false)}>
    <p class="muted">This removes your Vodka code, your contacts and every message you sent or received from the chat server. It can't be undone. Your Roblox account is not touched.</p>
    {#snippet actions()}
      <Button onclick={() => (confirmDelete = false)}>Cancel</Button>
      <Button variant="danger" onclick={() => turnOff(true)}>Delete chat account</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  .chat {
    height: 100%;
    padding-right: var(--s-4);
    display: grid;
    align-content: start;
  }
  .chat:has(.panes) {
    align-content: stretch;
  }
  .intro {
    display: grid;
    justify-items: start;
    gap: var(--s-3);
    max-width: 640px;
    padding: var(--s-6);
    border-radius: var(--r-xl);
  }
  .badge {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: var(--r-lg);
    background: var(--glass-strong);
    color: var(--frost);
    box-shadow: var(--glow);
  }
  .intro > ul {
    display: grid;
    gap: var(--s-2);
    margin: var(--s-2) 0;
    padding: 0;
    list-style: none;
    font-size: var(--fs-sm);
    color: var(--text-1);
  }
  .intro > ul li {
    display: flex;
    gap: var(--s-2);
    align-items: flex-start;
  }
  .intro > ul li :global(svg) {
    color: var(--frost);
    margin-top: 2px;
  }
  .verify {
    display: grid;
    gap: var(--s-2);
    width: 100%;
    padding: var(--s-4);
    border-radius: var(--r-md);
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--line);
    font-size: var(--fs-sm);
  }
  .verify ol {
    margin: 0;
    padding-left: var(--s-5);
    color: var(--text-1);
    display: grid;
    gap: 4px;
  }
  .phrase {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-2) var(--s-2) var(--s-2) var(--s-3);
    border-radius: var(--r-sm);
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--line-strong);
  }
  .phrase code {
    flex: 1;
    color: var(--frost);
    font-size: var(--fs-md);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    margin-top: var(--s-2);
  }
  .err {
    color: var(--danger);
    font-size: var(--fs-sm);
  }
  .err.small {
    padding: 0 var(--s-4) var(--s-2);
    font-size: var(--fs-xs);
  }
  .panes {
    display: grid;
    grid-template-columns: 260px 1fr;
    gap: var(--s-3);
    min-height: 0;
    height: 100%;
  }
  aside {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    padding: var(--s-3);
    min-height: 0;
  }
  .mine {
    display: grid;
    gap: 4px;
  }
  .label {
    font-size: var(--fs-xs);
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-1);
  }
  .tiny {
    font-size: 0.6875rem;
  }
  .codebtn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px var(--s-3);
    border-radius: var(--r-sm);
    border: 1px solid var(--line-strong);
    background: rgba(0, 0, 0, 0.35);
    color: var(--frost);
    cursor: pointer;
  }
  .codebtn:hover {
    background: var(--glass-hover);
  }
  .codebtn code {
    font-size: var(--fs-md);
    letter-spacing: 0.04em;
  }
  .add {
    display: flex;
    gap: var(--s-2);
  }
  input,
  textarea {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 var(--s-3);
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: rgba(0, 0, 0, 0.35);
    color: var(--text-0);
    font-size: var(--fs-sm);
    user-select: text;
  }
  textarea {
    width: 100%;
    height: auto;
    padding: var(--s-3);
    margin-top: var(--s-3);
    resize: vertical;
  }
  input:focus-visible,
  textarea:focus-visible {
    outline: none;
    border-color: rgba(164, 222, 255, 0.5);
    box-shadow: 0 0 0 3px rgba(98, 198, 255, 0.14);
  }
  .add input {
    height: 30px;
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
  }
  .empty {
    font-size: var(--fs-sm);
    padding: var(--s-3) 0;
  }
  .contacts {
    flex: 1;
    display: grid;
    align-content: start;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow-y: auto;
  }
  .contacts button {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    width: 100%;
    padding: 6px var(--s-2);
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .contacts button:hover {
    background: var(--glass-hover);
  }
  .contacts button.selected {
    background: var(--glass-strong);
    box-shadow: inset 0 0 0 1px var(--line-strong);
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
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  small {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .contacts i {
    min-width: 18px;
    padding: 0 5px;
    border-radius: var(--r-full);
    background: var(--frost);
    color: var(--frost-ink);
    font-size: 0.6875rem;
    font-style: normal;
    font-weight: 700;
    text-align: center;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    gap: var(--s-2);
    padding-top: var(--s-2);
    border-top: 1px solid var(--line);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-2);
    font-size: 0.6875rem;
    cursor: pointer;
  }
  .link:hover {
    color: var(--text-0);
    text-decoration: underline;
  }
  .link.danger:hover {
    color: var(--danger);
  }
  .thread {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .pick {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: var(--s-2);
    color: var(--text-2);
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4);
    border-bottom: 1px solid var(--line);
  }
  .menuwrap {
    position: relative;
  }
  .iconbtn {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: var(--text-1);
    cursor: pointer;
  }
  .iconbtn:hover {
    background: var(--glass-hover);
    color: var(--text-0);
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 3;
    min-width: 170px;
    padding: 6px;
    background: #141b24;
    border: 1px solid var(--line-strong);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-2);
  }
  .menu button {
    display: block;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    font-size: var(--fs-sm);
    text-align: left;
    cursor: pointer;
  }
  .menu button:hover {
    background: var(--glass-hover);
  }
  .messages {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    padding: var(--s-4);
    overflow-y: auto;
  }
  .hello {
    margin: auto;
    font-size: var(--fs-sm);
  }
  .msg {
    display: grid;
    justify-items: start;
    gap: 2px;
    max-width: 72%;
  }
  .msg.own {
    align-self: flex-end;
    justify-items: end;
  }
  .msg p {
    padding: 8px 12px;
    border-radius: 14px 14px 14px 4px;
    background: var(--glass-strong);
    font-size: var(--fs-sm);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .msg.own p {
    border-radius: 14px 14px 4px 14px;
    background: linear-gradient(180deg, #bfe7ff, var(--frost-strong));
    color: var(--frost-ink);
  }
  time {
    font-size: 0.625rem;
    color: var(--text-2);
  }
  .invite {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-2) var(--s-3) var(--s-2) var(--s-2);
    border-radius: var(--r-md);
    border: 1px solid rgba(164, 222, 255, 0.4);
    background: var(--glass-hover);
  }
  .invite img {
    width: 46px;
    height: 46px;
    border-radius: var(--r-sm);
  }
  .invite div {
    display: grid;
    line-height: 1.25;
  }
  .invite b {
    font-size: var(--fs-sm);
  }
  .composer {
    display: flex;
    gap: var(--s-2);
    padding: var(--s-3) var(--s-4);
    border-top: 1px solid var(--line);
  }
  .composer input {
    height: 38px;
  }
</style>
