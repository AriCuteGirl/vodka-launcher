# Changelog

## 0.1.0 (unreleased)

Phase 4 (in progress): social.

- Up to 10 games at once instead of 2: games beyond the second run from their own copy of Mocktail's binary. A five-second reminder shows each time another game is added.
- Marketplace: search and filter the avatar catalog, with prices, details, and copy ID or link.
- Buying from the Marketplace: fixed-price items only, behind a confirmation that shows the price and your balance. The purchase is refused if the price changed since you looked.
- Fixed: Settings showed "500 Internal Error" because the Performance form reloaded itself in a loop.
- The avatar picture and new outfit pictures refresh by themselves.
- Avatar editor: see what you are wearing, take items off, wear items you own, save the look as an outfit, and an Advanced box for up to ten accessories by item ID.
- Favorite, Like and Dislike buttons in each game's dialog.
- The party follows its leader into games automatically; members can turn following off.
- Several Vodka windows can run at once, each on its own account, sharing game slots safely.
- Parties: up to six Vodka users with a shared chat; the leader can bring everyone into the game they are playing.
- Avatar page: see your current look and switch between saved outfits.
- A running game's name and icon follow you when you switch games inside Roblox.
- Vodka chat (opt-in): message other Vodka users and invite them to your game, through Vodka's own server. Contacts are added by Vodka code; block, report and account deletion are built in.
- Server picker on every game tile: let Roblox choose, fewest players, a specific public server, or a private server you can join.
- Tick several accounts on Players and the next Play or Join starts for all of them, in the same server when one is chosen.
- Memory safety limit is a preset list starting at 4 GB, and a game closed by the limit says so.
- Games page: search all of Roblox and browse the public charts, with Play on every tile.
- Friends page: friends grouped by what they are doing, a filter, friend requests with Accept and Decline, and Join for friends whose server is visible.
- The right rail lists friends who are online.
- Pressing Enter in the title-strip search searches all of Roblox.

Phase 3: performance and tweaks.

- Launch options for all accounts or one: frame rate, graphics backend, VSync, present mode, multithreaded rendering, GameMode, memory cap.
- FFlag editor with name and value checks, presets, JSON import and export, and reset.
- Live FPS for each running game, measured from Mocktail's frame counter.
- Per-account Roblox client version, "hold updates", and one-step rollback through Mocktail's updater.

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
