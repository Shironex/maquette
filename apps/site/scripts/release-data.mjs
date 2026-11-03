/**
 * Turns a GitHub release into the numbers the Download card shows. Pure and
 * synchronous apart from `loadReleaseData`, which takes its fetch as an
 * argument so tests never touch the network.
 */

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const DMG = /_aarch64\.dmg$/;
const SETUP = /_x64-setup\.exe$/;

/** "2026-11-03T18:48:23Z" as "3 Nov 2026" (UTC, no locale tables). */
export function formatReleaseDate(iso) {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) throw new Error(`not a date: ${iso}`);
  return `${at.getUTCDate()} ${MONTHS[at.getUTCMonth()]} ${at.getUTCFullYear()}`;
}

function sizeOf(assets, pattern, label) {
  const hits = assets.filter(asset => pattern.test(asset?.name ?? ''));
  if (hits.length !== 1) throw new Error(`expected one ${label} asset, found ${hits.length}`);
  const size = hits[0].size;
  if (!Number.isInteger(size) || size <= 0) throw new Error(`${label} asset has no size`);
  return size;
}

/** The REST payload of `releases/latest` as the card's data. Throws on anything unexpected. */
export function parseRelease(payload) {
  if (!payload || typeof payload !== 'object') throw new Error('empty release payload');
  if (payload.draft || payload.prerelease) throw new Error('release is a draft or a pre-release');
  const version = String(payload.tag_name ?? '').replace(/^v/, '');
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error(`unexpected tag ${payload.tag_name}`);
  const assets = Array.isArray(payload.assets) ? payload.assets : [];
  return {
    version,
    date: formatReleaseDate(payload.published_at),
    macBytes: sizeOf(assets, DMG, '.dmg'),
    windowsBytes: sizeOf(assets, SETUP, '-setup.exe'),
  };
}

/**
 * Reads the latest release, or says why it could not. Never throws: a build
 * with no network, no release or a rate limit keeps the checked-in values.
 * Returns `{ data, source, reason? }` with source `'github'` or `'fallback'`.
 */
export async function loadReleaseData({ fallback, repo, token, fetchImpl = fetch }) {
  try {
    const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'maquette-site-build' };
    if (token) headers.Authorization = `Bearer ${token}`;
    const res = await fetchImpl(`https://api.github.com/repos/${repo}/releases/latest`, {
      headers,
      signal: AbortSignal.timeout(15_000),
    });
    if (!res.ok) throw new Error(`GitHub answered ${res.status}`);
    return { data: parseRelease(await res.json()), source: 'github' };
  } catch (error) {
    return {
      data: fallback,
      source: 'fallback',
      reason: error instanceof Error ? error.message : String(error),
    };
  }
}
