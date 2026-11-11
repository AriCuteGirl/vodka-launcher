# Changelog

## 0.1.0 (unreleased)

Phase 2: accounts and instances.

- Several Roblox accounts: add, switch and remove them on the Players page or from the title strip. Each session has its own keyring entry.
- Each account plays in its own Mocktail profile (data, cache, state). The Roblox client files are hard-linked, so a profile costs almost no disk space.
- Vodka signs Mocktail in as the selected account and deletes Mocktail's copy of the session when the game closes.
- Up to two games at once, one per account, behind a one-time warning.
- Running games show CPU and memory, with Restart and Close.
- Account presence (online, in a game) on the Players page.

Phase 1: first vertical slice.

- App shell: icon sidebar, title strip with search, back/forward and account menu, right rail.
- Detects the Mocktail Flatpak, its version, the Roblox client version, and whether it is running.
- Guided install steps per distro when Flatpak or Mocktail is missing.
- Three-step first-run setup.
- Sign in through Roblox's own login page in a private window; the session is stored in the system keyring only.
- Home: hero, "Continue playing" and "Favorites" rows with thumbnails, and Play buttons that start Mocktail in that game.
- Launch output and crash details are shown in the app.
- Roblox API client limited to roblox.com hosts, with caching and rate-limit backoff.
