# Build "Vodka Launcher": a polished Roblox launcher for Linux

You're my senior engineer and designer. We're building **Vodka Launcher**: a fast, good-looking desktop launcher for Roblox on Linux. Think of what Lunar Client is doing for Roblox (lunar.gg/roblox), which is Windows/macOS only and not out yet, but native to Linux and built on top of **Mocktail**.

Follow my CLAUDE.md rules for everything: ask first, plan before you build, verify by actually running it, keep reports short.

---

## 1. Context you need first

**Mocktail** (https://github.com/komaruworld/mocktail, Apache-2.0) is what I use now to play Roblox on Linux. It's an experimental compatibility runtime written in C++17. It runs the **Android** Roblox client by providing Bionic/JNI pieces and wiring them to SDL3 + Vulkan/OpenGL. Facts that matter:
- It downloads the Roblox APK on first launch, verifies its signature and ABI, and keeps the last working APK so a bad update can be rolled back.
- FFlag overrides live in `$XDG_CONFIG_HOME/mocktail/fflags.json` and apply on next launch.
- Its own UI is GTK4 + libadwaita + WebKitGTK. It's packaged as Flatpak, AUR, DEB, RPM, and AppImage.
- It doesn't work with `hardened_malloc`.

**Lunar for Roblox** (the inspiration) advertises: unlimited isolated instances, uncapped FPS, the full catalog and trades, macros, a live second-screen HUD, a multi-account Players view, friends, and a home/discovery screen. The UI is a dark space theme: a slim left icon sidebar, a top bar with search and an account switcher, a main content grid, and a right rail with friends and requests.

**Important:** take inspiration from the *feature set and layout ideas* only. Don't copy Lunar's name, logo, mascot, art, or copy. Vodka needs its own identity.

---

## 2. Architecture direction (confirm with me before committing)

Vodka is a **launcher/frontend that drives Mocktail**. It is **not** a fork of Mocktail's runtime. Don't rewrite or vendor Mocktail's C++ runtime. Treat it as the engine:
- Detect the installed Mocktail (Flatpak, AUR/system binary, or AppImage). If it isn't there, offer guided install.
- Launch Mocktail as a child process per instance. Capture its logs, exit codes, and crashes and show them nicely.
- Before you design the integration, **read Mocktail's source** (`src/`, `config/`, `exe/`, README, FAQ) to find out what it actually supports: CLI args, env vars, config/data dirs, deep links / `roblox://` launch URIs, FFlags, multiple windows. **Don't invent flags.** If something we need doesn't exist, tell me and propose either a small upstream PR to Mocktail or a workaround.
- Mocktail is open source, so no reverse engineering is needed. Read the code.

Ask me which stack to use. Give me a short trade-off table for:
1. **Tauri 2 (Rust backend + Svelte or React + TypeScript frontend)** (my likely pick: small binary, easy to make it look custom and beautiful)
2. GTK4 + libadwaita (matches Mocktail, native, but harder to get a custom Lunar-style look)
3. Electron (easiest UI work, but heavy)

---

## 3. Features (build as phases, one polished vertical slice at a time)

### Phase 0: Audit and plan (no code yet)
- Clone Mocktail and summarize how it launches, where it stores data, which config it accepts, and what's realistically possible for instances, FPS, and launching into a specific game or server.
- List which Roblox web APIs each feature needs (catalog, economy, trades, friends, presence, games, thumbnails, users) and which of them need an authenticated session.
- Flag anything risky (see section 5). Propose the phase plan and wait for my OK.

### Phase 1: Vertical slice
- Polished app shell: left icon sidebar (Home, Games, Players, Marketplace, Friends, Settings), top bar (search, back/forward, account switcher), right rail.
- Mocktail detection plus a first-run onboarding that takes under 30 seconds.
- Sign in with one Roblox account through an embedded login webview. Store the session **only in the system keyring** (libsecret / Secret Service), never in plaintext files or logs.
- Home: "Continue playing" and "Favorites" rows with real thumbnails, and a Play button that launches Mocktail into that experience.

### Phase 2: Instances and accounts
- **Multi-account manager** (Players view): add, remove, and switch accounts, each with its own avatar and presence.
- **Isolated instances**: each instance gets its own data/config dirs (for example, per-profile `XDG_CONFIG_HOME` / `XDG_DATA_HOME`, or whatever Mocktail supports) so sessions don't collide. Show running instances with CPU/RAM use, and add kill and restart buttons. Find out first whether running several Mocktail processes at once actually works.

### Phase 3: Performance and tweaks
- FPS cap/uncap and graphics presets, implemented through the FFlags Mocktail supports. Check which FFlags the current Roblox client still honors before exposing them. Use a safe FFlag editor with validation, presets, and reset-to-default.
- Per-instance launch options. APK version pinning and rollback, using Mocktail's existing mechanism.

### Phase 4: Social and marketplace
- Friends: online/in-game presence, friend requests (accept/decline), and "join friend" where possible.
- Marketplace/catalog: browse, search, filter, and view item details with thumbnails. Outfits view.
- Trades: view inbound and outbound trades read-only first. Ask me before adding anything that sends or accepts trades or spends Robux. Those actions always need an explicit confirm dialog.

### Phase 5: Extras
- **Second-screen HUD**: a separate always-on-top or detachable window showing session time, FPS (if Mocktail exposes it), instance status, and friend activity.
- **Macros**: see section 5 before designing these.
- Discord Rich Presence (optional, ask me first).
- Auto-update for Vodka itself, plus packaging as AppImage, Flatpak, and AUR `PKGBUILD`.

---

## 4. Design

Before designing, ask me for vibe, colors, and references, or propose 2–3 directions. My starting idea: **dark, premium, "frosted glass bottle" feel**. Deep near-black background, a cold icy accent (frost blue or clear crystal), subtle grain, soft glows, smooth hover motion. It should be clearly its own brand, not a Lunar clone.

- Build the design system first: color tokens, type scale, spacing, radius, shadows, and components (sidebar item, card, game tile, avatar, button variants, input, toast, modal, skeleton).
- Design every state: loading skeletons, empty, error (including "Mocktail not installed" and "Roblox API rate-limited"), offline.
- Keyboard navigation, visible focus rings, `prefers-reduced-motion`, WCAG AA contrast.
- Make an original logo and favicon (simple SVG). Write real copy, no lorem ipsum.
- Run the app and screenshot each screen. Fix anything that looks off before showing me.

---

## 5. Safety, ToS, and security (non-negotiable)

- **Roblox session cookies are account keys.** Keep them in the keyring, never log them, never send them anywhere except official `*.roblox.com` endpoints, and add no telemetry. Validate every URL the app opens. Lock down the webview (CSP, no remote code, allowlisted domains).
- **Macros**: launcher-side automation (auto-launch, auto-rejoin, scheduled launches, account rotation for launching) is fine. In-game input automation can break Roblox's Terms of Use and individual games' rules. Explain the risk to me and ask before building any of that. No exploits, executors, script injection, or anything that touches Roblox's anti-cheat.
- Respect Roblox API rate limits: cache, back off, and show friendly errors.
- Respect Mocktail's Apache-2.0 license: attribution in About/NOTICE, and don't bundle the Roblox APK.
- Add a clear disclaimer: Vodka is not affiliated with Roblox Corporation, Mocktail, or Lunar.

---

## 6. Engineering bar

- Clean module boundaries: `mocktail` (detect/launch/process management), `instances`, `accounts` (keyring), `roblox-api` (typed client, caching, rate limiting), `ui`.
- No swallowed errors. Every failure gets a useful message in the UI.
- Tests for core logic: Mocktail detection, instance dir isolation, FFlag validation, API response parsing.
- Pinned dependencies. Tell me before adding any.
- README with screenshots and install/run/build steps, `.env.example` if needed, CHANGELOG, and a LICENSE (ask me which one).
- Small git commits with clear messages.

---

## Start here

1. Read Mocktail's repo and give me the Phase 0 audit (short and factual: what's confirmed vs. guessed).
2. Ask me your questions (stack, design direction, license, distro targets) with AskUserQuestion, at most 4 at a time.
3. Propose the plan for Phase 1 and wait for my OK.
