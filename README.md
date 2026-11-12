# Vodka Launcher

A Roblox launcher for Linux. Vodka is the front end; [Mocktail](https://github.com/komaruworld/mocktail) is the engine that runs the game.

![First-run setup](docs/screenshots/onboarding.png)

**Status:** phase 4 of 5, in progress. Home, Games, Friends, several accounts, up to two games at once, launch options and an FFlag editor exist. Marketplace is a placeholder.

## What it does today

- Finds the Mocktail Flatpak (`space.bigrat.mocktail`) and tells you how to install it if it's missing.
- Signs you in on Roblox's own login page. The session goes into the system keyring (Secret Service) and nowhere else.
- Shows "Continue playing" and "Favorites" with a Play button that starts Mocktail in that game.
- Shows Mocktail's output while it starts, and the reason if it crashes.
- Keeps several accounts, each with its own isolated Mocktail profile, and switches between them.
- Runs up to two games at once, one per account, with FPS, CPU and memory per game.
- Searches all of Roblox and shows the public charts.
- Shows your friends, what they're playing, and friend requests; joins a friend's server when Roblox shares it.
- Sets frame rate, graphics and FFlags for all accounts or per account, and can hold or roll back the Roblox client.

Running two Roblox clients at once is not supported by Mocktail's developers and is against Roblox's Terms of Use. Vodka warns
before the first time; the risk to your accounts is yours.

Not yet: AUR, system-package and AppImage installs of Mocktail are not detected.

## Requirements

- Linux with Flatpak and Mocktail (`flatpak install flathub space.bigrat.mocktail`)
- A running Secret Service keyring (GNOME Keyring, KWallet, KeePassXC)
- To build: Rust 1.88+, Node 22+, pnpm, and Tauri's Linux dependencies (`webkit2gtk-4.1`)

## Run and build

```bash
pnpm install
pnpm tauri dev      # run the desktop app
pnpm tauri build    # build an AppImage
```

```bash
pnpm dev            # UI only in a browser, with demo data
pnpm check          # type-check
pnpm test           # UI unit tests
cd src-tauri && cargo test   # backend tests
```

The browser-only UI accepts `?demo=` to show a state: `loading`, `empty`, `error`, `ratelimited`, `offline`, `expired`,
`nomocktail`, `noflatpak`, `running`, `crash`, `signedout`, `onboarding`.

## Layout

| Path | What |
|---|---|
| `src-tauri/src/mocktail` | Find Mocktail, start it, read its output and exit |
| `src-tauri/src/accounts` | Login window and keyring storage, one entry per account |
| `src-tauri/src/instances` | Running games: per-account profiles, slots, CPU/RAM, stop and restart |
| `src-tauri/src/roblox_api` | Typed Roblox web API client: host allowlist, cache, backoff |
| `src-tauri/src/tweaks.rs` | Launch options and FFlag validation, mapped to Mocktail's environment variables |
| `src-tauri/src/settings.rs` | Non-secret preferences |
| `src/lib` | UI: design tokens, components, state |
| `src/routes` | Pages |

## Privacy and safety

- Vodka contacts `*.roblox.com` only, plus Roblox's image CDN for thumbnails. No telemetry.
- Your Roblox session is an account key. Vodka keeps it in the keyring, never logs it, and hides any log line that contains one.
- No exploits, executors, script injection, or anything that touches Roblox's anti-cheat.

## License and credits

Apache-2.0, see [LICENSE](LICENSE) and [NOTICE](NOTICE). Vodka runs Mocktail (Apache-2.0, Mocktail Project Authors) as a separate
program and does not bundle it or the Roblox client.

Vodka Launcher is an independent project. It is not affiliated with Roblox Corporation, Mocktail, or Lunar Client.
