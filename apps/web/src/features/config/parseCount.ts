/** Reads "50k", "50,000" or "50000". */
export function parseCount(text: string): number | null {
  const t = text
    .trim()
    .toLowerCase()
    .replace(/[,\s_]/g, '');
  const match = t.match(/^(\d+(?:\.\d+)?)(k)?$/);
  if (!match) return null;
  const n = Number(match[1]) * (match[2] ? 1000 : 1);
  return Number.isFinite(n) ? Math.round(n) : null;
}
