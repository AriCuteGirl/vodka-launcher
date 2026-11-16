# Vodka Launcher for Windows: preview, please test

This is an early Windows build. Most of it has only been tried in a virtual machine, and Roblox refuses to run in
one, so **nobody has joined a game with it on Windows yet.** That is the main thing to test.

Vodka is not affiliated with Roblox Corporation.

## Install

1. You need Windows 10 or 11 with Roblox already installed (from roblox.com, the normal way).
2. Unzip `vodka-launcher-windows.zip` anywhere. Keep `Vodka Launcher.exe` and `WebView2Loader.dll` in the same folder.
3. Run `Vodka Launcher.exe`. The program isn't signed, so Windows SmartScreen may warn you: click
   **More info → Run anyway**.

Your Roblox session is stored in Windows Credential Manager (the same place Windows keeps other app logins) and is
only ever sent to roblox.com.

## Already checked (in a Windows 10 VM)

- The app starts.
- It finds the installed Roblox and shows its version.
- Signing in through Roblox's own login page works (Quick Sign-in was used).
- The session is saved and still there after closing and reopening Vodka.
- Home, Friends, Avatar, Marketplace and the Chat page load with real data.
- Pressing Play gets a one-time sign-in ticket from Roblox and hands the game link to Windows.

## Please test, in this order

Use your own account. Write down what happened for each step, plus any error text (screenshots help).

1. **Play a game from Home.** Does Roblox open, already signed in as you, and put you in that game?
2. **Vodka while you play.** Within about a minute of being in the game, does the side panel switch from "Starting"
   to showing the game as running? Does it show the right game?
3. **Leave the game** (close Roblox). Within about a minute, does Vodka notice the game ended?
4. **Join a friend** from the Friends list while they are in a game. Do you land in their server?
5. **A game that doesn't start.** If Roblox fails to open, Vodka should say so after about 4 minutes. Does it?
6. **Close from Vodka.** Start a game, then press Close in Vodka's side panel. Does Roblox close?
7. **Restart Vodka** after signing in. Are you still signed in?

## Several accounts at once (optional, risky)

Off by default. **Read the warning before you turn it on.** Roblox's Terms of Use don't allow working around its
one-client limit, Roblox is actively blocking tools that do it, and accounts that do it can be restricted or banned.
Only try it with accounts you're willing to lose. Don't use automation or exploits with it.

1. Close every Roblox window.
2. In Vodka: **Settings → Several games at once → Turn on…**, read the warning, and accept it.
3. Add a second account (**Players → Add account**).
4. Play a game on the first account. Switch to the second account (top right) and press Play again. Vodka shows a
   reminder each time another game starts.
5. Do both Roblox windows open and stay open? Is each one signed in as the right account?
6. Close one game from Vodka's side panel. Does only that window close?
7. If the second window closes the first one, or won't open, write down exactly what happened.

## Game pages, servers, private servers, groups (new)

1. Click a game's picture (not its Play button). Does a page open with the banner, title, Play, likes, favorite and
   an About section? Does **Play** on the page start the game?
2. Press **Servers**. Do the server cards show? Does **Join fewest players** put you in a small server?
3. Press **Private servers**. Are friends' private servers listed? If you own one, do **Invite only / Friends can
   join**, the on/off switch and **Invite link** work?
4. On a game with **free** private servers, press **Create free server**. Does it show up under **Yours**? Can
   your friends join it? (Only try a paid one if you're happy to spend the Robux; the dialog shows the price first.)
5. On a game made by a group you're not in, press **Join (group name)**. Does it say you joined, with no window?
   If a Roblox window pops up, write down what it showed.

## Luduvo (new)

The rail on the far left switches to Luduvo. On Windows you can sign in, browse, see friends and game pages, but
**Play doesn't start Luduvo yet**: Vodka only knows where the Linux version is installed. Check that browsing and
sign-in work (sign-in opens luduvo.com in your normal browser), and tell the owner where Luduvo is installed.

## Closing games and performance settings (new)

1. Start two games on two accounts. Close **one** Roblox window yourself (the X). Within a few seconds, does Vodka's
   side panel drop only that game?
2. Press **Close** in Vodka's side panel on the other game. Does exactly that window close?
3. Does each running game show its memory use (like "1.2 GB") in the side panel?
4. **Settings → Performance**: set Frame rate to **Unlimited** and press Save, then start a game. Does it run above
   60 FPS (Shift+F5 in Roblox shows FPS)?
5. Set **Graphics API** to **Vulkan** (or OpenGL), start a game. Shift+F2 in Roblox shows which one it uses.
6. Add a FastFlag (for example `FFlagDebugDisplayFPS` = `True`), start a game. Vodka writes it into
   `%LOCALAPPDATA%\Roblox\Versions\<version>\ClientSettings\ClientAppSettings.json`. Is it there?
7. Set a **Memory safety limit** of 4 GB. Vodka closes a game that goes over it and says so.

## Not in the Windows version (on purpose)

- No per-game CPU or FPS readout, Restart or Gather. These need Mocktail, which is Linux-only.
- No graphics backend, VSync, GameMode or update holding: those are Mocktail options.

## If something goes wrong

Send the owner what you did, what you expected, what happened, and any error text. Closing Vodka never closes a game
already running in Roblox, and nothing in Vodka spends Robux without a confirmation that shows the price.
