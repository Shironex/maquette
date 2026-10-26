/** "2.3 GB", "840 KB", "0 B". Decimal units, one decimal from MB up. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = unit >= 2 ? 1 : 0;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

/** "just now", "4 s ago", "2 min ago", "3 h ago", "6 days ago". */
export function formatAgo(thenMs: number, nowMs: number): string {
  const s = Math.max(0, Math.round((nowMs - thenMs) / 1000));
  if (s < 3) return 'just now';
  if (s < 60) return `${s} s ago`;
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 48) return `${h} h ago`;
  return `${Math.round(h / 24)} days ago`;
}

/** "09:05", the local time of day of `ms`. */
export function formatClock(ms: number): string {
  const at = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(at.getHours())}:${pad(at.getMinutes())}`;
}

/** "60,000" */
export function formatCount(n: number): string {
  return Math.round(n).toLocaleString('en-US');
}

/** "50k" for 50000, "1.5k" for 1500, "800" below a thousand. */
export function formatK(n: number): string {
  if (n < 1000) return String(Math.round(n));
  const k = n / 1000;
  return `${Number.isInteger(k) ? k : k.toFixed(1)}k`;
}

/** `/Users/me/Projects/x` reads as `~/Projects/x`; other paths stay as they are. */
export function homePath(path: string, home: string | null | undefined): string {
  if (!home) return path;
  const trimmed = home.replace(/[\\/]+$/, '');
  if (path === trimmed) return '~';
  for (const sep of ['/', '\\']) {
    if (path.startsWith(trimmed + sep)) return `~${sep}${path.slice(trimmed.length + 1)}`;
  }
  return path;
}

/** The last few segments of a path, for tight rows: "sites/moon/public" stays readable. */
export function shortPath(path: string, segments = 3): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  if (parts.length <= segments) return path;
  return parts.slice(-segments).join('/');
}
