# Vodka Launcher for Windows

Vodka is a launcher for Roblox and Luduvo. Vodka is not affiliated with Roblox Corporation or Luduvo Corporation.

## Install

1. Windows 10 or 11. For Roblox, install Roblox from roblox.com the normal way. For Luduvo, install it with the
   installer from luduvo.com/download (Vodka has a button for it on the Luduvo page).
2. Unzip the Vodka zip anywhere you like. Keep `Vodka Launcher.exe` and `WebView2Loader.dll` in the same folder.
3. Run `Vodka Launcher.exe`. The program isn't signed, so Windows SmartScreen may warn you the first time: click
   **More info → Run anyway**.

Your Roblox and Luduvo sessions are kept in Windows Credential Manager (where Windows keeps other app logins), and are
only ever sent to roblox.com and luduvo.com.

## Roblox

- **Play** a game from Home, a game's page, or join a friend from the Friends panel.
- **Game pages:** click a game's picture for its page, its servers (join the fewest or busiest) and private servers
  (friends' servers, your own, invite links, and making new ones; paid ones always show the price first).
- **Groups:** join a game's group from its page.
- **Profiles, Friends, Avatar, Marketplace, Chat** pages.
- **Settings → Performance:** frame rate, graphics API (Direct3D 11, Vulkan, OpenGL), FastFlags and a memory safety
  limit. Vodka writes them into Roblox's `ClientAppSettings.json` and keeps flags it didn't write.
- Vodka follows each Roblox window it started: closing a window removes only that game, and **Close** in the side
  panel closes the right one.

## Luduvo

The rail on the far left switches to Luduvo.

- **Play** starts the game through Luduvo's own launcher. If Luduvo updates itself or downloads game content first,
  Vodka shows a progress bar. Vodka then shows the game as running, counts playtime, and **Close game** closes it.
- **Open Studio** opens Luduvo Studio.
- **Marketplace, Avatar** (outfits, colours), **Friends, Messages** (with gifts) and profiles.
- Your Lunes balance is at the top; click it to buy more on luduvo.com.

## Several Roblox accounts at once (optional, risky)

Off by default. **Read the warning before you turn it on.** Roblox's Terms of Use don't allow working around its
one-client limit, Roblox is actively blocking tools that do it, and accounts that do it can be restricted or banned.
Only use it with accounts you're willing to lose. Turn it on in **Settings → Several games at once**.

## Not in the Windows version (on purpose)

- No per-game CPU or FPS readout, Restart or Gather for Roblox. These need Mocktail, which is Linux-only.
- No graphics backend, VSync, GameMode or update holding for Roblox: those are Mocktail options.

## If something goes wrong

Open an issue on GitHub with what you did, what you expected, what happened, and any error text (a screenshot helps).
Closing Vodka never closes a game that's already running, and nothing in Vodka spends Robux or Lunes without a
confirmation that shows the price.
