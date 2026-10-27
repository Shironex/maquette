/**
 * Tests for the release scripts: `pnpm test:scripts`.
 */
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { after, describe, test } from 'node:test';

import { checkVersions, normalize, readVersions, setVersion } from './bump-version.mjs';

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
