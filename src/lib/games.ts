// The games Vodka can launch. Each owns a part of the app: its pages, its accounts and its accent colour.
import type { IconName } from "./components/Icon.svelte";
import type { GameId } from "./types";

export interface GameSection {
  id: GameId;
  name: string;
  /** Where the game's part of Vodka starts. */
  home: string;
  pages: { href: string; label: string; icon: IconName }[];
}

export const GAMES: GameSection[] = [
  {
    id: "roblox",
    name: "Roblox",
    home: "/",
    pages: [
      { href: "/", label: "Home", icon: "home" },
      { href: "/games", label: "Games", icon: "games" },
      { href: "/players", label: "Players", icon: "players" },
      { href: "/avatar", label: "Avatar", icon: "avatar" },
      { href: "/marketplace", label: "Marketplace", icon: "market" },
      { href: "/friends", label: "Friends", icon: "friends" },
      { href: "/chat", label: "Chat", icon: "chat" },
      { href: "/playtime", label: "Playtime", icon: "clock" },
    ],
  },
  {
    id: "luduvo",
    name: "Luduvo",
    home: "/luduvo",
    pages: [
      { href: "/luduvo", label: "Home", icon: "home" },
      { href: "/luduvo/games", label: "Games", icon: "games" },
      { href: "/luduvo/avatar", label: "Avatar", icon: "avatar" },
      { href: "/luduvo/marketplace", label: "Marketplace", icon: "market" },
      { href: "/luduvo/friends", label: "Friends", icon: "friends" },
      { href: "/luduvo/playtime", label: "Playtime", icon: "clock" },
    ],
  },
];

/** Which game a page belongs to. Settings and other shared pages count as Roblox's for now. */
export function gameOf(pathname: string): GameSection {
  return GAMES.find((g) => g.id !== "roblox" && (pathname === g.home || pathname.startsWith(g.home + "/"))) ?? GAMES[0];
}

/** The installed app can start on an empty path instead of "/"; both mean Roblox Home. */
export function pathOf(pathname: string): string {
  return pathname || "/";
}
