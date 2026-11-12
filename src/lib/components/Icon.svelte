<script module lang="ts">
  // Vodka's own icon set: 24px grid, 1.7 stroke, round joins.
  const PATHS = {
    home: "M4 11.200 12 4l8 7.200V19a1 1 0 0 1-1 1h-4.500v-5.500h-5V20H5a1 1 0 0 1-1-1v-7.800Z",
    games: "M7.500 8h9a4.500 4.500 0 0 1 4.400 5.400l-.6 3a2.600 2.600 0 0 1-4.700 1L14.500 16h-5l-1.100 1.400a2.600 2.600 0 0 1-4.700-1l-.6-3A4.500 4.500 0 0 1 7.500 8ZM8 11v3M6.500 12.500h3M15.500 11.500h.01M17.500 13.500h.01",
    players: "M9 11a3.200 3.200 0 1 0 0-6.400A3.200 3.200 0 0 0 9 11ZM3.500 19.500c0-3 2.500-5 5.500-5s5.500 2 5.500 5M16 5a3 3 0 0 1 0 5.800M17.500 14.800c1.800.6 3 2.300 3 4.700",
    market: "M5 8h14l-1 11a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1L5 8ZM9 8V7a3 3 0 0 1 6 0v1",
    friends: "M12 20s-7-4.300-7-9.500A4 4 0 0 1 12 8a4 4 0 0 1 7 2.500c0 5.200-7 9.500-7 9.500Z",
    settings: "M4 7h9M17 7h3M4 12h3M11 12h9M4 17h11M19 17h1M15 5v4M9 10v4M17 15v4",
    search: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14ZM20 20l-4-4",
    back: "M14.500 6 8.500 12l6 6",
    forward: "M9.500 6l6 6-6 6",
    play: "M8 5.500v13l11-6.500L8 5.500Z",
    check: "M5 12.500l4.500 4.500L19 7.500",
    copy: "M9 9h10v11H9zM5 15V4h10",
    close: "M6 6l12 12M18 6 6 18",
    alert: "M12 4 2.500 20h19L12 4ZM12 10v4.500M12 17.500h.01",
    wifiOff: "M3 3l18 18M8.500 16.500a5 5 0 0 1 7 0M5 13a10 10 0 0 1 4-2.600M19 13a10 10 0 0 0-5.500-2.900M2 9.500A15 15 0 0 1 6.500 6.500M22 9.500a15 15 0 0 0-10.500-4M12 20h.01",
    clock: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18ZM12 7.500V12l3 2",
    star: "M12 4l2.500 5.200 5.700.8-4.100 4 1 5.700L12 17l-5.100 2.700 1-5.700-4.100-4 5.700-.8L12 4Z",
    refresh: "M19.500 12a7.500 7.500 0 1 1-2.200-5.300M19.500 4.500v4h-4",
    signOut: "M10 4H6a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h4M15 8l4 4-4 4M19 12H10",
    plus: "M12 5v14M5 12h14",
    terminal: "M4 6h16v12H4zM7.500 10l2.500 2-2.500 2M12.500 14h4",
    bottle: "M10 3h4v3.500c0 .8.3 1.500.9 2.100A5.500 5.500 0 0 1 16.500 12.500V19a2 2 0 0 1-2 2h-5a2 2 0 0 1-2-2v-6.500c0-1.500.6-2.900 1.600-3.900.6-.6.9-1.300.9-2.100V3Z",
    lock: "M7 11V8a5 5 0 0 1 10 0v3M6 11h12v9H6z",
    chevron: "M7 10l5 5 5-5",
    avatar: "M12 11.500a3.500 3.500 0 1 0 0-7 3.500 3.500 0 0 0 0 7ZM5.500 20c0-3.300 2.900-5.500 6.500-5.500s6.500 2.200 6.500 5.500",
    menu: "M4 7h16M4 12h16M4 17h16",
    chat: "M5 5h14a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-7l-4.500 3.500V16H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1Z",
    send: "M4.500 12 20 4.500 14.500 20l-3-6.500-7-1.500Z",
    more: "M6 12h.01M12 12h.01M18 12h.01",
  } as const;

  export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
  let { name, size = 20, filled = false }: { name: IconName; size?: number; filled?: boolean } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={filled ? "currentColor" : "none"}
  stroke="currentColor"
  stroke-width="1.7"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={PATHS[name]} />
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>
