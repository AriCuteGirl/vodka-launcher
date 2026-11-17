<script lang="ts">
  import { untrack } from "svelte";
  import { api } from "$lib/api";
  import { applyPreset, parseFlagJson, PRESETS } from "$lib/presets";
  import { app, onWindows, rollbackClient, saveLaunchOptions, toast } from "$lib/state.svelte";
  import { NO_OPTIONS, toAppError, type LaunchOptions } from "$lib/types";
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  /** null = every account; otherwise one account's overrides. */
  let scope = $state<number | null>(null);
  let draft = $state<LaunchOptions>({ ...NO_OPTIONS });
  let dirty = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let flagName = $state("");
  let flagValue = $state("");
  let flagError = $state<string | null>(null);
  let importing = $state(false);
  let importText = $state("");
  let importError = $state<string | null>(null);

  const saved = (s: number | null): LaunchOptions => (s == null ? app.settings.defaults : (app.settings.accountOptions[String(s)] ?? NO_OPTIONS));
  // On Windows the official client runs the game: only the options it can take are shown.
  const win = $derived(onWindows());
  const inherit = $derived(scope == null ? (win ? "Roblox's setting" : "Mocktail's setting") : "Same as all accounts");
  const account = $derived(app.accounts.find((a) => a.id === scope) ?? null);
  const flagRows = $derived(Object.entries(draft.fflags).sort(([a], [b]) => a.localeCompare(b)));
  const MEMORY_PRESETS = [4096, 6144, 8192, 12288, 16384];
  /** The preset to show for a saved value; older or hand-edited values round up to the next preset. */
  const memoryChoice = $derived(
    draft.memoryLimitMb == null ? "" : draft.memoryLimitMb === 0 ? "0" : String(MEMORY_PRESETS.find((mb) => mb >= draft.memoryLimitMb!) ?? MEMORY_PRESETS.at(-1)),
  );
  const fpsChoice = $derived(draft.fps == null ? "" : draft.fps === "display" || draft.fps === "unlimited" ? draft.fps : "custom");

  function load(s: number | null) {
    scope = s;
    const from = saved(s);
    const next: LaunchOptions = { ...NO_OPTIONS, ...from, fflags: { ...from.fflags } };
    let changed = false;
    // A limit saved by an older Vodka may be lower than today's minimum: show the raised value and offer to save it.
    const mb = next.memoryLimitMb;
    if (mb != null && mb !== 0 && !MEMORY_PRESETS.includes(mb)) {
      next.memoryLimitMb = MEMORY_PRESETS.find((p) => p >= mb) ?? MEMORY_PRESETS[MEMORY_PRESETS.length - 1];
      changed = true;
    }
    draft = next;
    dirty = changed;
    error = null;
    flagError = null;
  }
  // Reload when saved settings change underneath (for example after a rollback).
  $effect(() => {
    void app.settings;
    // Only saved settings changing should reload the form. Without `untrack`, the form's own
    // state would count as a trigger and the reload would re-run itself forever.
    untrack(() => {
      if (!dirty) load(scope);
    });
  });

  function set<K extends keyof LaunchOptions>(key: K, value: LaunchOptions[K]) {
    draft[key] = value;
    dirty = true;
  }
  const text = (v: string) => (v === "" ? null : v);
  const tri = (v: string) => (v === "" ? null : v === "on");

  async function addFlag() {
    flagError = null;
    try {
      const [name, value] = await api.checkFflag(flagName, flagValue);
      draft.fflags = { ...draft.fflags, [name]: value };
      dirty = true;
      flagName = "";
      flagValue = "";
    } catch (e) {
      flagError = toAppError(e).message;
    }
  }

  function removeFlag(name: string) {
    const { [name]: _, ...rest } = draft.fflags;
    draft.fflags = rest;
    dirty = true;
  }

  async function save() {
    saving = true;
    const empty = JSON.stringify(draft) === JSON.stringify(NO_OPTIONS);
    error = await saveLaunchOptions(scope, scope != null && empty ? null : $state.snapshot(draft));
    saving = false;
    // A rejected save must be impossible to miss: nothing was stored.
    if (error) toast(`Not saved. ${error}`, "danger");
    if (!error) {
      dirty = false;
      toast("Saved. It applies the next time a game starts.", "ok");
    }
  }

  function reset() {
    draft = { ...NO_OPTIONS, fflags: {} };
    dirty = true;
  }

  function doImport() {
    const parsed = parseFlagJson(importText);
    if ("error" in parsed) {
      importError = parsed.error;
      return;
    }
    draft.fflags = { ...draft.fflags, ...parsed.flags };
    dirty = true;
    importing = false;
    importText = "";
    toast(`Added ${Object.keys(parsed.flags).length} flags. They are checked when you save.`);
  }

  async function exportFlags() {
    try {
      await navigator.clipboard.writeText(JSON.stringify(draft.fflags, null, 2));
      toast("Flags copied to the clipboard as JSON.", "ok");
    } catch {
      toast("Couldn't reach the clipboard.", "danger");
    }
  }
</script>

<div class="tweaks">
  <div class="tabs" role="tablist" aria-label="Whose settings">
    <button role="tab" aria-selected={scope == null} onclick={() => load(null)}>All accounts</button>
    {#each app.accounts as a (a.id)}
      <button role="tab" aria-selected={scope === a.id} onclick={() => load(a.id)}>{a.displayName}</button>
    {/each}
  </div>
  <p class="faint">
    {#if scope == null}
      Used for every account unless that account sets its own. Anything left on "{inherit}" stays
      {win ? "as Roblox has it. Vodka writes these into Roblox's ClientAppSettings.json just before each game starts." : "under the control of Mocktail's config.yaml."}
    {:else}
      Only for {account?.displayName}. Anything left on "{inherit}" follows the All accounts tab.
    {/if}
  </p>

  <div class="grid">
    <label>
      <span>Frame rate</span>
      <select value={fpsChoice} onchange={(e) => set("fps", e.currentTarget.value === "custom" ? "120" : text(e.currentTarget.value))}>
        <option value="">{inherit}</option>
        <option value="display">Match the display</option>
        <option value="unlimited">{win ? "Unlimited" : "Unlimited (up to 240)"}</option>
        <option value="custom">Custom limit</option>
      </select>
    </label>
    {#if fpsChoice === "custom"}
      <label>
        <span>FPS limit</span>
        <input type="number" min="1" max="1000" value={draft.fps} oninput={(e) => set("fps", e.currentTarget.value)} />
      </label>
    {/if}
    {#if !win}
    <label>
      <span>Graphics backend</span>
      <select value={draft.backend ?? ""} onchange={(e) => set("backend", text(e.currentTarget.value))}>
        <option value="">{inherit}</option>
        <option value="direct-vulkan">Vulkan (direct)</option>
        <option value="angle-vulkan">Vulkan through ANGLE</option>
        <option value="opengl">OpenGL</option>
        <option value="system">System default</option>
      </select>
    </label>
    {/if}
    {#if !win}
    <label>
      <span>VSync</span>
      <select value={draft.vsync ?? ""} onchange={(e) => set("vsync", text(e.currentTarget.value))}>
        <option value="">{inherit}</option>
        <option value="auto">Automatic</option>
        <option value="on">On</option>
        <option value="off">Off</option>
      </select>
    </label>
    {/if}
    {#if !win}
    <label>
      <span>Present mode</span>
      <select value={draft.presentMode ?? ""} onchange={(e) => set("presentMode", text(e.currentTarget.value))}>
        <option value="">{inherit}</option>
        <option value="auto">Automatic</option>
        <option value="fifo">FIFO (no tearing)</option>
        <option value="mailbox">Mailbox (low latency)</option>
        <option value="immediate">Immediate (may tear)</option>
      </select>
    </label>
    {/if}
    {#if !win}
    <label>
      <span>Multithreaded rendering</span>
      <select
        value={draft.multithreadedRendering == null ? "" : draft.multithreadedRendering ? "on" : "off"}
        onchange={(e) => set("multithreadedRendering", tri(e.currentTarget.value))}
      >
        <option value="">{inherit}</option>
        <option value="on">On</option>
        <option value="off">Off</option>
      </select>
    </label>
    {/if}
    {#if !win}
    <label>
      <span>Feral GameMode</span>
      <select value={draft.gamemode ?? ""} onchange={(e) => set("gamemode", text(e.currentTarget.value))}>
        <option value="">{inherit}</option>
        <option value="auto">When available</option>
        <option value="on">Always</option>
        <option value="off">Never</option>
      </select>
    </label>
    {/if}
    {#if win}
      <label>
        <span>Graphics API</span>
        <select value={draft.renderer ?? ""} onchange={(e) => set("renderer", text(e.currentTarget.value))}>
          <option value="">{inherit}</option>
          <option value="auto">Let Roblox choose</option>
          <option value="d3d11">Direct3D 11</option>
          <option value="vulkan">Vulkan</option>
          <option value="opengl">OpenGL</option>
        </select>
        <small>Sets Roblox's renderer flag. Roblox falls back to its own choice if your graphics card can't use it.</small>
      </label>
    {/if}
    <label>
      <span>Memory safety limit</span>
      <select value={memoryChoice} onchange={(e) => set("memoryLimitMb", e.currentTarget.value === "" ? null : Number(e.currentTarget.value))}>
        <option value="">{inherit}</option>
        <option value="0">Off (recommended)</option>
        {#each MEMORY_PRESETS as mb (mb)}
          <option value={String(mb)}>{mb / 1024} GB</option>
        {/each}
      </select>
      <small>Doesn't make Roblox use less. {win ? "Vodka" : "Mocktail"} closes the game if it goes over, so leave room for spikes.</small>
    </label>
    {#if !win}
    <label>
      <span>If the game crashes or disconnects</span>
      <select value={draft.autoRejoin == null ? "" : draft.autoRejoin ? "on" : "off"} onchange={(e) => set("autoRejoin", tri(e.currentTarget.value))}>
        <option value="">{scope == null ? "Do nothing" : inherit}</option>
        <option value="on">Rejoin automatically</option>
        {#if scope != null}<option value="off">Do nothing</option>{/if}
      </select>
      <small>Starts the same game again, up to 5 times in 15 minutes. Closing a game yourself never triggers it.</small>
    </label>
    {/if}
    {#if !win}
    <label>
      <span>Roblox client updates</span>
      <select value={draft.holdUpdates == null ? "" : draft.holdUpdates ? "on" : "off"} onchange={(e) => set("holdUpdates", tri(e.currentTarget.value))}>
        <option value="">{inherit}</option>
        <option value="off">Update on launch</option>
        <option value="on">Hold the current version</option>
      </select>
    </label>
    {/if}
  </div>

  {#if account && !win}
    <div class="client">
      <span class="muted">
        {account.clientVersion ? `This account's profile runs Roblox ${account.clientVersion}.` : "This account hasn't played here yet."}
      </span>
      <Button size="sm" disabled={!account.clientVersion || account.inVodkaGame} onclick={() => rollbackClient(account.id)}>Roll back one version</Button>
    </div>
  {/if}

  <h3>Presets</h3>
  <div class="presets">
    {#each PRESETS as p (p.id)}
      <button
        class="preset"
        onclick={() => {
          draft = applyPreset($state.snapshot(draft), p);
          dirty = true;
        }}
      >
        <b>{p.name}</b>
        <span>{p.description}</span>
        <em class:ok={p.verified}>{p.verified ? "Effect measured" : "Passed to Roblox, effect not verified"}</em>
      </button>
    {/each}
  </div>

  <h3>FFlags <span class="count">{flagRows.length}</span></h3>
  <p class="faint">
    Client settings sent to Roblox on launch. Roblox may ignore a flag without saying so, and flags used to gain an unfair advantage can get an
    account actioned.
  </p>
  {#if flagRows.length}
    <ul class="flags">
      {#each flagRows as [name, value] (name)}
        <li>
          <code class="selectable">{name}</code>
          <code class="value selectable">{value}</code>
          <button onclick={() => removeFlag(name)} aria-label="Remove {name}"><Icon name="close" size={14} /></button>
        </li>
      {/each}
    </ul>
  {/if}
  <form
    class="add"
    onsubmit={(e) => {
      e.preventDefault();
      void addFlag();
    }}
  >
    <input bind:value={flagName} placeholder="FFlagName" spellcheck="false" autocomplete="off" aria-label="Flag name" aria-invalid={!!flagError} />
    <input bind:value={flagValue} placeholder="Value" spellcheck="false" autocomplete="off" aria-label="Flag value" />
    <Button type="submit" icon="plus" disabled={!flagName.trim() || !flagValue.trim()}>Add</Button>
  </form>
  {#if flagError}<p class="err" role="alert">{flagError}</p>{/if}

  <div class="foot">
    <Button variant="ghost" onclick={() => (importing = true)}>Import JSON</Button>
    <Button variant="ghost" disabled={!flagRows.length} onclick={exportFlags}>Copy as JSON</Button>
    <Button variant="ghost" onclick={reset}>Reset to default</Button>
    <span class="grow"></span>
    {#if dirty}<span class="faint">Unsaved changes</span>{/if}
    <Button disabled={!dirty} onclick={() => load(scope)}>Discard</Button>
    <Button variant="primary" disabled={!dirty} busy={saving} onclick={save}>Save</Button>
  </div>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</div>

{#if importing}
  <Modal title="Import FFlags" onclose={() => (importing = false)}>
    <p class="muted">Paste a JSON object, for example the contents of a fflags.json file.</p>
    <textarea bind:value={importText} rows="8" spellcheck="false" placeholder={'{\n  "FFlagExample": "True"\n}'}></textarea>
    {#if importError}<p class="err" role="alert">{importError}</p>{/if}
    {#snippet actions()}
      <Button onclick={() => (importing = false)}>Cancel</Button>
      <Button variant="primary" disabled={!importText.trim()} onclick={doImport}>Add flags</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  .tweaks {
    display: grid;
    gap: var(--s-4);
  }
  p {
    font-size: var(--fs-sm);
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 4px;
    width: fit-content;
    border-radius: var(--r-md);
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--line);
  }
  .tabs button {
    padding: 6px 14px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: var(--text-1);
    font-size: var(--fs-sm);
    font-weight: 600;
    cursor: pointer;
  }
  .tabs button:hover {
    color: var(--text-0);
  }
  .tabs button[aria-selected="true"] {
    background: var(--glass-strong);
    color: var(--frost);
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--s-3) var(--s-4);
  }
  label {
    display: grid;
    gap: 6px;
  }
  label span {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-1);
  }
  label small {
    font-size: 0.6875rem;
    color: var(--text-2);
  }

  select,
  input,
  textarea {
    width: 100%;
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
    height: auto;
    padding: var(--s-3);
    margin-top: var(--s-3);
    font-family: var(--font-mono);
    resize: vertical;
  }
  select:focus-visible,
  input:focus-visible,
  textarea:focus-visible {
    outline: none;
    border-color: rgba(164, 222, 255, 0.5);
    box-shadow: 0 0 0 3px rgba(98, 198, 255, 0.14);
  }
  input[aria-invalid="true"] {
    border-color: rgba(255, 155, 155, 0.6);
  }
  option {
    background: #141b24;
  }
  .client {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4);
    border-radius: var(--r-md);
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--line);
    font-size: var(--fs-sm);
  }
  h3 {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    margin: var(--s-2) 0 0;
    font-size: var(--fs-md);
    font-weight: 600;
  }
  .count {
    font-size: var(--fs-xs);
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--glass-hover);
    color: var(--text-1);
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--s-3);
  }
  .preset {
    display: grid;
    gap: 4px;
    align-content: start;
    padding: var(--s-3) var(--s-4);
    border-radius: var(--r-md);
    border: 1px solid var(--line);
    background: var(--glass);
    text-align: left;
    cursor: pointer;
    transition:
      border-color var(--fast) var(--ease),
      background var(--fast) var(--ease),
      transform var(--fast) var(--ease);
  }
  .preset:hover {
    background: var(--glass-hover);
    border-color: rgba(164, 222, 255, 0.45);
    transform: translateY(-2px);
  }
  .preset span {
    font-size: var(--fs-sm);
    color: var(--text-1);
  }
  .preset em {
    margin-top: 4px;
    font-style: normal;
    font-size: 0.6875rem;
    color: var(--warn);
  }
  .preset em.ok {
    color: var(--ok);
  }
  .flags {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .flags li {
    display: grid;
    grid-template-columns: 1fr minmax(80px, 30%) auto;
    align-items: center;
    gap: var(--s-3);
    padding: 6px 6px 6px var(--s-3);
    border-radius: var(--r-sm);
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--line);
  }
  .flags code {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .value {
    color: var(--frost);
  }
  .flags button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .flags button:hover {
    background: var(--glass-hover);
    color: var(--danger);
  }
  .add {
    display: grid;
    grid-template-columns: 1fr minmax(80px, 30%) auto;
    gap: var(--s-2);
  }
  .add input {
    font-family: var(--font-mono);
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-2);
    padding-top: var(--s-3);
    border-top: 1px solid var(--line);
  }
  .grow {
    flex: 1;
  }
  .err {
    color: var(--danger);
  }
</style>
