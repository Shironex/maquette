import assert from 'node:assert/strict';
import { test } from 'node:test';
import { formatReleaseDate, loadReleaseData, parseRelease } from './release-data.mjs';

const payload = {
  tag_name: 'v0.7.0',
  draft: false,
  prerelease: false,
  published_at: '2026-11-03T18:48:23Z',
  assets: [
    { name: 'latest.json', size: 900 },
    { name: 'Maquette_0.7.0_aarch64.dmg', size: 21687653 },
    { name: 'Maquette_0.7.0_aarch64.app.tar.gz', size: 19000000 },
    { name: 'Maquette_0.7.0_x64-setup.exe', size: 17092911 },
    { name: 'Maquette_0.7.0_x64-setup.exe.sig', size: 400 },
  ],
};
const fallback = { version: '0.6.0', date: '1 Nov 2026', macBytes: 1, windowsBytes: 2 };

test('formats the date in UTC', () => {
  assert.equal(formatReleaseDate('2026-11-03T18:48:23Z'), '3 Nov 2026');
  assert.equal(formatReleaseDate('2026-12-31T23:59:59Z'), '31 Dec 2026');
  assert.throws(() => formatReleaseDate('nope'));
});

test('parses version, date and the two installer sizes', () => {
  assert.deepEqual(parseRelease(payload), {
    version: '0.7.0',
    date: '3 Nov 2026',
    macBytes: 21687653,
    windowsBytes: 17092911,
  });
});

test('rejects a release it cannot use', () => {
  assert.throws(() => parseRelease({ ...payload, draft: true }));
  assert.throws(() => parseRelease({ ...payload, prerelease: true }));
  assert.throws(() => parseRelease({ ...payload, tag_name: 'nightly' }));
  assert.throws(
    () => parseRelease({ ...payload, assets: payload.assets.slice(0, 2) }),
    /-setup\.exe/
  );
  assert.throws(() => parseRelease(null));
});

const reply = (ok, status, body) => async () => ({ ok, status, json: async () => body });

test('loads from the API response', async () => {
  const seen = [];
  const fetchImpl = async (url, init) => {
    seen.push([url, init.headers.Authorization]);
    return reply(true, 200, payload)();
  };
  const out = await loadReleaseData({ fallback, repo: 'Shironex/maquette', token: 't', fetchImpl });
  assert.equal(out.source, 'github');
  assert.equal(out.data.version, '0.7.0');
  assert.deepEqual(seen, [
    ['https://api.github.com/repos/Shironex/maquette/releases/latest', 'Bearer t'],
  ]);
});

test('falls back on a failed request, a bad status or a bad payload', async () => {
  const boom = async () => {
    throw new Error('offline');
  };
  for (const fetchImpl of [
    boom,
    reply(false, 403, {}),
    reply(true, 200, { tag_name: 'v1.0.0', assets: [] }),
  ]) {
    const out = await loadReleaseData({ fallback, repo: 'a/b', fetchImpl });
    assert.equal(out.source, 'fallback');
    assert.equal(out.data, fallback);
    assert.ok(out.reason);
  }
});
