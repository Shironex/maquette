/**
 * Tests for the release scripts: `pnpm test:scripts`.
 * The signature tests need minisign on the PATH and skip without it.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { after, describe, test } from 'node:test';

import { checkVersions, normalize, readVersions, setVersion } from './bump-version.mjs';
import { buildManifest } from './latest-json.mjs';
import { readNotes } from './release-notes.mjs';
import { hasMinisign, shippedPubkey, verifyDir } from './verify-signatures.mjs';

const REPO = resolve(import.meta.dirname, '../..');
const scratch = mkdtempSync(join(tmpdir(), 'maquette-release-test-'));
after(() => rmSync(scratch, { recursive: true, force: true }));

let counter = 0;
function tempDir() {
  const dir = join(scratch, String(counter++));
  mkdirSync(dir, { recursive: true });
  return dir;
}

function put(root, rel, text) {
  mkdirSync(dirname(join(root, rel)), { recursive: true });
  writeFileSync(join(root, rel), text);
}

/** A tiny copy of the repo's version files, all at `version`. */
function fakeRepo(version = '0.1.0', eol = '\n') {
  const root = tempDir();
  put(
    root,
    'package.json',
    `{\n  "name": "maquette",\n  "version": "${version}",\n  "private": true\n}\n`
  );
  put(
    root,
    'apps/desktop/package.json',
    `{\n  "name": "@maquette/desktop",\n  "version": "${version}",\n  "devDependencies": { "x": "1.0.0" }\n}\n`
  );
  put(
    root,
    'apps/desktop/src-tauri/tauri.conf.json',
    `{\n  "productName": "Maquette",\n  "version": "${version}",\n  "plugins": { "updater": { "version": "keep" } }\n}\n`
  );
  put(
    root,
    'Cargo.toml',
    `[workspace]\nmembers = ["crates/*"]\n\n[workspace.package]\nversion = "${version}"\nedition = "2024"\n\n[workspace.dependencies]\nserde = { version = "1" }\n`
  );
  put(
    root,
    'crates/maquette-core/Cargo.toml',
    '[package]\nname = "maquette-core"\nversion.workspace = true\n'
  );
  put(
    root,
    'apps/desktop/src-tauri/Cargo.toml',
    '[package]\nname = "maquette-desktop"\nversion.workspace = true\n'
  );
  const lock = [
    '[[package]]',
    'name = "maquette-core"',
    `version = "${version}"`,
    '',
    '[[package]]',
    'name = "maquette-desktop"',
    `version = "${version}"`,
    '',
    '[[package]]',
    'name = "serde"',
    'version = "1.0.200"',
    '',
  ].join(eol);
  put(root, 'Cargo.lock', lock);
  return root;
}

describe('bump-version', () => {
  test('normalize accepts a tag and refuses anything else', () => {
    assert.equal(normalize('v1.2.3'), '1.2.3');
    assert.equal(normalize('1.2.3-rc.1'), '1.2.3-rc.1');
    assert.throws(() => normalize('1.2'), /Not a version/);
    assert.throws(() => normalize('main'), /Not a version/);
    assert.throws(() => normalize(undefined), /Not a version/);
  });

  test('sets every file and leaves everything else alone', () => {
    const root = fakeRepo('0.1.0');
    const changed = setVersion(root, 'v0.2.0');
    assert.deepEqual(changed.sort(), [
      'Cargo.lock',
      'Cargo.toml',
      'apps/desktop/package.json',
      'apps/desktop/src-tauri/tauri.conf.json',
      'package.json',
    ]);
    assert.deepEqual(checkVersions(root, '0.2.0').problems, []);
    const toml = readFileSync(join(root, 'Cargo.toml'), 'utf8');
    assert.match(toml, /serde = \{ version = "1" \}/);
    const conf = readFileSync(join(root, 'apps/desktop/src-tauri/tauri.conf.json'), 'utf8');
    assert.match(conf, /"version": "keep"/);
    assert.match(readFileSync(join(root, 'Cargo.lock'), 'utf8'), /version = "1\.0\.200"/);
    assert.deepEqual(setVersion(root, '0.2.0'), [], 'a second run changes nothing');
  });

  test('keeps CRLF line endings in Cargo.lock', () => {
    const root = fakeRepo('0.1.0', '\r\n');
    setVersion(root, '0.3.0');
    const lock = readFileSync(join(root, 'Cargo.lock'), 'utf8');
    assert.match(lock, /name = "maquette-desktop"\r\nversion = "0\.3\.0"/);
    assert.deepEqual(checkVersions(root, '0.3.0').problems, []);
  });

  test('check names the file that disagrees and the expected version', () => {
    const root = fakeRepo('0.1.0');
    put(
      root,
      'apps/desktop/package.json',
      '{\n  "name": "@maquette/desktop",\n  "version": "0.0.9"\n}\n'
    );
    const { problems, version } = checkVersions(root);
    assert.equal(version, null);
    assert.match(problems.join('\n'), /apps\/desktop\/package\.json = 0\.0\.9/);
    const tagged = checkVersions(fakeRepo('0.1.0'), 'v0.2.0');
    assert.deepEqual(tagged.problems, ['the files say 0.1.0, expected 0.2.0']);
  });

  test('the repo itself carries one version everywhere', () => {
    const { problems } = checkVersions(REPO);
    assert.deepEqual(problems, []);
    assert.ok(readVersions(REPO).length >= 6, 'four files plus the crates in Cargo.lock');
  });
});

describe('release-notes', () => {
  test('takes the title from the heading and the rest as notes', () => {
    const root = tempDir();
    put(
      root,
      'docs/releases/v0.2.0.md',
      '# v0.2.0: Motion view\r\n\r\nShort intro.\r\n\r\n## Highlights\r\n'
    );
    const notes = readNotes(root, 'v0.2.0');
    assert.equal(notes.title, 'v0.2.0: Motion view');
    assert.equal(notes.body, 'Short intro.\n\n## Highlights');
    assert.equal(notes.source, 'docs/releases/v0.2.0.md');
  });

  test('falls back to a default without the file', () => {
    const notes = readNotes(tempDir(), '0.2.0');
    assert.equal(notes.title, 'v0.2.0');
    assert.equal(notes.source, null);
    assert.match(notes.body, /Maquette 0\.2\.0/);
  });
});

describe('latest-json', () => {
  function releaseDir() {
    const dir = tempDir();
    put(dir, 'Maquette_0.2.0_aarch64.app.tar.gz', 'mac');
    put(dir, 'Maquette_0.2.0_aarch64.app.tar.gz.sig', 'bWFjIHNpZw==\n');
    put(dir, 'Maquette_0.2.0_x64-setup.exe', 'win');
    put(dir, 'Maquette_0.2.0_x64-setup.exe.sig', 'd2luIHNpZw==');
    put(dir, 'Maquette_0.2.0_aarch64.dmg', 'dmg');
    return dir;
  }

  test('lists both platforms with their signatures and release URLs', () => {
    const manifest = buildManifest({
      dir: releaseDir(),
      version: 'v0.2.0',
      repo: 'Shironex/maquette',
      notes: 'Notes.',
      pubDate: new Date('2026-11-03T12:00:00Z'),
    });
    assert.deepEqual(manifest, {
      version: '0.2.0',
      notes: 'Notes.',
      pub_date: '2026-11-03T12:00:00.000Z',
      platforms: {
        'darwin-aarch64': {
          signature: 'bWFjIHNpZw==',
          url: 'https://github.com/Shironex/maquette/releases/download/v0.2.0/Maquette_0.2.0_aarch64.app.tar.gz',
        },
        'windows-x86_64': {
          signature: 'd2luIHNpZw==',
          url: 'https://github.com/Shironex/maquette/releases/download/v0.2.0/Maquette_0.2.0_x64-setup.exe',
        },
      },
    });
  });

  test('refuses a release without a signature', () => {
    const dir = releaseDir();
    rmSync(join(dir, 'Maquette_0.2.0_x64-setup.exe.sig'));
    assert.throws(
      () => buildManifest({ dir, version: '0.2.0', repo: 'o/r', notes: '' }),
      /windows-x86_64: Maquette_0\.2\.0_x64-setup\.exe\.sig is missing/
    );
  });
});

describe('verify-signatures', () => {
  test('reads the shipped public key', () => {
    assert.match(
      Buffer.from(shippedPubkey(REPO), 'base64').toString('utf8'),
      /^untrusted comment: minisign public key/
    );
  });

  const skip = hasMinisign() ? false : 'minisign is not on the PATH';

  /** A throwaway key and a signed release folder in Tauri's base64 format. */
  function signedRelease() {
    const keys = tempDir();
    const gen = spawnSync(
      'minisign',
      ['-G', '-W', '-f', '-p', join(keys, 'k.pub'), '-s', join(keys, 'k.key')],
      { encoding: 'utf8' }
    );
    assert.equal(gen.status, 0, gen.stderr);
    const dir = tempDir();
    for (const name of ['Maquette_0.2.0_aarch64.app.tar.gz', 'Maquette_0.2.0_x64-setup.exe']) {
      put(dir, name, `bytes of ${name}`);
      const sig = join(keys, `${name}.minisig`);
      const run = spawnSync(
        'minisign',
        ['-S', '-s', join(keys, 'k.key'), '-m', join(dir, name), '-x', sig],
        { encoding: 'utf8' }
      );
      assert.equal(run.status, 0, run.stderr);
      put(dir, `${name}.sig`, readFileSync(sig).toString('base64'));
    }
    const pubkey = readFileSync(join(keys, 'k.pub')).toString('base64');
    return { dir, pubkey };
  }

  test('accepts a release signed with the key', { skip }, () => {
    const { dir, pubkey } = signedRelease();
    const manifest = buildManifest({ dir, version: '0.2.0', repo: 'o/r', notes: '' });
    put(dir, 'latest.json', JSON.stringify(manifest));
    assert.deepEqual(verifyDir(dir, { pubkey, version: '0.2.0' }), []);
  });

  test('rejects a changed file, a stale latest.json and another key', { skip }, () => {
    const { dir, pubkey } = signedRelease();
    const manifest = buildManifest({ dir, version: '0.2.0', repo: 'o/r', notes: '' });
    manifest.platforms['darwin-aarch64'].signature = 'c3RhbGU=';
    put(dir, 'latest.json', JSON.stringify(manifest));
    put(dir, 'Maquette_0.2.0_x64-setup.exe', 'tampered');
    const problems = verifyDir(dir, { pubkey, version: '0.3.0' }).join('\n');
    assert.match(problems, /Maquette_0\.2\.0_x64-setup\.exe\.sig: does not verify/);
    assert.match(problems, /the darwin-aarch64 signature is not the one/);
    assert.match(problems, /latest\.json says version 0\.2\.0, expected 0\.3\.0/);

    const other = signedRelease();
    assert.match(verifyDir(dir, { pubkey: other.pubkey }).join('\n'), /does not verify/);
    assert.match(verifyDir(other.dir, { pubkey }).join('\n'), /does not verify/);
  });
});
