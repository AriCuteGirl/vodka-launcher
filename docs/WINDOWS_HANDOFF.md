# Finishing the Windows version: handoff

This is the brief for whoever continues the Windows port. Read `docs/claude.md` first; its rules still apply
(ask first, plan before building, verify by actually running it, short reports, pinned dependencies, small commits).

## Where things stand

Vodka Launcher is a Roblox launcher. On Linux it drives **Mocktail** (a runtime for the Android Roblox client) and is
complete through phase 4: accounts, up to 10 games at once, launch options, friends, its own chat and parties, avatar
editor, marketplace with buying, auto-rejoin, gather. All of that is confirmed working on Linux by the owner.

The Windows version exists as a **first cut that compiles and has never been run on Windows**.

- Mocktail does not exist on Windows. There the "engine" is the **official Roblox client**.
- The owner chose **"core first"**: accounts, launching as the chosen account, Games, Friends, Chat and parties,
  Avatar, Marketplace. Multi-instance was added later at the owner's request, off by default and behind two warnings
  (see below). Don't extend it to anything that opens or changes Roblox processes.

## What was built for Windows

| File | What it does |
|---|---|
| `src-tauri/src/instances_windows.rs` | Windows engine with the same public API as `instances/` (Linux). One game at a time. |
| `src-tauri/src/windows_player.rs` | Pure functions: read Vodka's launch link, build the `roblox-player:` link, pick the newest installed client. Tested on Linux. |
| `src-tauri/src/mocktail/detect.rs` | `detect()` has a `#[cfg(windows)]` version; `MocktailStatus.engine` is `"mocktail"` or `"roblox"`. |
| `src-tauri/src/roblox_api/mod.rs` | `auth_ticket()`: one-time sign-in ticket from `auth.roblox.com/v1/authentication-ticket`. |
| `src-tauri/src/lib.rs` | `#[cfg]` switch between the two engines; rollback disabled on Windows. |
| `src-tauri/Cargo.toml` | `reqwest` uses rustls on Linux and native TLS (SChannel) on Windows, because `aws-lc-sys` won't cross-build with mingw. |
| `scripts/build-windows.sh` | Cross-builds from Linux and writes `dist/vodka-launcher-windows.zip`. |
| UI (`onWindows()` in `src/lib/state.svelte.ts`) | Says "Roblox" instead of "Mocktail", hides Performance settings, per-game stats, Restart and Gather. |

How a Windows launch works:
1. Read the account's session from the keyring (Windows Credential Manager through the `keyring` crate).
2. Ask Roblox for a one-time sign-in ticket.
3. Build `roblox-player:1+launchmode:play+gameinfo:<ticket>+launchtime:<ms>+placelauncherurl:<encoded PlaceLauncher URL>+browsertrackerid:<n>+robloxLocale:en_us+gameLocale:en_us+channel:`.
4. Hand it to Windows with `rundll32.exe url.dll,FileProtocolHandler <link>`.
5. There is no child process to watch (Roblox's launcher hands off and exits), so a game's start and end are learned
   from the account's own presence, polled every 30 s by `refresh_instance_games`.

## What is verified and what is not

Verified on Linux:
- The Windows target compiles with zero warnings; the Linux build and its 103 tests are unaffected.
- `auth_ticket()` returns a real ticket for a real account (448 characters, URL-safe).
- The `.exe` imports only system DLLs plus `WebView2Loader.dll`, which ships next to it.

Verified in the `itch-test` VM (Windows 10 22H2, WebView2 157) on 2026-11-15:
- The app starts and renders (WebView2 and `WebView2Loader.dll` found). Item 1 below is done.
- Before Roblox is installed, onboarding says so. After the official installer runs, it shows "Roblox found" with
  the version. The `Versions\*\RobloxPlayerBeta.exe` layout and the `roblox-player` handler are as expected. Item 7 is done.
- How to drive the VM: `guest-exec` runs as SYSTEM in session 0, so start GUI programs through a scheduled task with
  an `Interactive` principal for user `tester` (task `VodkaTest` starts `C:\vodka\app\Vodka Launcher.exe`).
  A single `guest-file-write` argument must stay under 128 KiB, so copy files in ~72 KiB chunks.
- Sign-in works (owner used Roblox Quick Sign-in). It first stayed blank on `about:blank`: `begin_login` was a sync
  command, which runs on the main thread, and building a webview window there deadlocks on Windows. It is async now.
  Keep every command that opens a window async. Item 2 is done.
- The session is stored as `roblox-session:<id>.vodka-launcher` in Credential Manager, it fits, and it is read back
  after a restart. Item 3 is done.
- Home, Friends, Avatar, Marketplace and the Chat intro load with real data.
- **Roblox refuses to run in a VM** ("Virtual Machine detected. Roblox can't be used in a Virtual Machine or Virtual
  Desktop"), so a real launch can only be tested on a real Windows PC. Don't try to hide the VM from Roblox.
  Pressing Play in the VM did get a sign-in ticket and hand the link to Windows; the client then quit with that error.
- A launch that never shows up in presence is dropped after 4 minutes and the user is now told why (it used to vanish
  silently). The status box says "Waiting for Roblox…" on Windows instead of "Waiting for Mocktail…".
- **Multi-instance (added 2026-11-15 at the owner's request, with two warnings):** off by default. Turning it on in
  Settings shows a warning (Terms of Use, Roblox actively blocking such tools, close Roblox first), and each extra game
  shows a reminder. While it is on, Vodka holds the named mutex `ROBLOX_singletonEvent` in its own process before the
  first client starts, as MultiBloxy does. Vodka never touches Roblox processes to free the lock; if Roblox was already
  open, the user is told to close it. One game per account. Closing a game kills only the client whose command line
  carries that launch's `browsertrackerid`. Verified in the VM: the lock is held when on and released when off. Not
  verified: whether current Roblox builds actually allow a second client this way (needs a real PC).
- Debugging the sign-in window: start Vodka with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`
  (task `VodkaDebug` does) and talk DevTools protocol to `127.0.0.1:9222` inside the guest.

**Still unknown, in rough order of risk:**
1. Whether the app starts at all (WebView2 present? `WebView2Loader.dll` found?).
2. Whether the login window works and `cookies_for_url` returns `.ROBLOSECURITY` under WebView2.
3. Whether the session fits in Windows Credential Manager. A credential blob is limited to 2560 bytes and the
   cookie is about 1150 characters; if it is stored as UTF-16 it may not fit. If it fails, split it across entries.
4. Whether the `roblox-player:` link format is still what the current client accepts, including
   `RequestGameJob` (specific server) and `RequestPrivateGame`.
5. Whether `rundll32 url.dll,FileProtocolHandler` passes a link this long intact.
6. Whether presence-based tracking feels right (a game shows "Starting" until presence says "in a game").
7. Detection: `%LOCALAPPDATA%\Roblox\Versions\*\RobloxPlayerBeta.exe`. Newer installs may use a different layout.

## Build

```bash
./scripts/build-windows.sh
```

Needs `rustup target add x86_64-pc-windows-gnu` and a mingw-w64 gcc. On this machine mingw comes from Nix and keeps
`libpthread.a` in a separate store path; the script finds it.

## Testing

Two libvirt VMs exist on the owner's PC: `itch-test` (`qemu:///system`, Windows 10, 10 GB RAM, no GPU acceleration)
and `win10` (`qemu:///session`). `itch-test` has the QEMU guest agent connected, SPICE graphics on 127.0.0.1:5900,
no IP reported by `domifaddr`, and ports 22/3389/5985/445 closed. So files and commands have to go through the guest
agent (`virsh qemu-agent-command` with `guest-file-*` and `guest-exec`) or SPICE; `virsh screenshot` shows the screen.

Without GPU acceleration Roblox itself will probably not run well in the VM. The realistic split:
- **In the VM:** app starts, onboarding, sign-in, keyring, Home/Games/Friends/Chat/Avatar/Marketplace load, and
  pressing Play hands a link to Windows (Roblox's launcher should at least appear).
- **On a real Windows PC with the owner:** an actual game join.

Rules that apply to testing: never type the owner's Roblox credentials; the owner signs in. Do not start games on the
owner's accounts unattended (an earlier project of theirs got an account banned after automated joins).

## Suggested order

1. Get the `.exe` into the VM, start it, fix whatever stops it from starting.
2. Sign-in and keyring (items 2 and 3 above).
3. Read-only pages.
4. Launch hand-off, then a real join with the owner.
5. Installer (NSIS or a zip), then attach a Windows build to a GitHub release.

Ask the owner before: adding dependencies, anything touching multi-instance on Windows, or publishing a Windows release.

## Things that are shared and must not break

- The chat and party server is a Supabase project (`server/supabase/`), reached at a fixed URL in `src-tauri/src/chat.rs`.
  It works the same from Windows. The Supabase connector in the Claude app was down when this was written.
- Linux is the platform the owner uses daily. Run `cargo test` in `src-tauri` and `pnpm check` before every commit,
  and rebuild/reinstall Linux with `./scripts/install-local.sh` if shared code changed.
