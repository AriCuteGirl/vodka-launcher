/** 149534 -> "149.5K", 2100000 -> "2.1M". */
export function compact(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(n < 10_000 ? 1 : 0).replace(/\.0$/, "")}K`;
  if (n < 1_000_000_000) return `${(n / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`;
  return `${(n / 1_000_000_000).toFixed(1).replace(/\.0$/, "")}B`;
}

/** Seconds -> "4m 05s" or "1h 12m". */
export function elapsed(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${String(m).padStart(2, "0")}m`;
  return `${m}m ${String(s % 60).padStart(2, "0")}s`;
}

/** Bytes -> "512 MB" or "1.4 GB". */
export function memory(bytes: number): string {
  const mb = bytes / 1024 / 1024;
  return mb < 1000 ? `${Math.round(mb)} MB` : `${(mb / 1024).toFixed(1)} GB`;
}

/** Case- and accent-insensitive "does this name match the search box". */
export function matches(name: string, query: string): boolean {
  const norm = (s: string) => s.normalize("NFKD").replace(/[^\p{L}\p{N} ]/gu, "").toLowerCase();
  const q = norm(query).trim();
  return q === "" || norm(name).includes(q);
}
