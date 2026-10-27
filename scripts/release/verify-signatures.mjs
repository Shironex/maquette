/**
 * Verifies a release's update signatures against the public key the app
 * ships with (`plugins.updater.pubkey` in tauri.conf.json), using minisign:
 *
 * - every `*.sig` in the folder verifies against the file it sits next to,
 * - every updater platform has a signed file,
 * - and when `latest.json` is there, it names those files, carries their
 *   signatures, and says the expected version.
 *
 *   node scripts/release/verify-signatures.mjs <dir> [--version 0.2.0]
 *
 * Tauri writes the key and each `.sig` as base64 of minisign's own text
 * format, so both are decoded into temporary files first. Needs `minisign` on
 * the PATH (apt, brew or scoop all have it).
 */
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { normalize } from './bump-version.mjs';
import { PLATFORMS } from './latest-json.mjs';

const ROOT = resolve(import.meta.dirname, '../..');

/** The updater public key the app trusts, as tauri.conf.json stores it (base64). */
export function shippedPubkey(root = ROOT) {
  const config = JSON.parse(
    readFileSync(join(root, 'apps/desktop/src-tauri/tauri.conf.json'), 'utf8')
  );
  const key = config?.plugins?.updater?.pubkey;
  if (!key) throw new Error('tauri.conf.json has no plugins.updater.pubkey');
  return key;
}

const decode = text => Buffer.from(text.trim(), 'base64').toString('utf8');

/** True when minisign can be run. */
export function hasMinisign() {
  return !spawnSync('minisign', ['-v'], { encoding: 'utf8' }).error;
}

/**
 * Checks `dir` and returns a list of problems (empty when every signature
 * holds). `pubkey` is the base64 key as tauri.conf.json stores it.
 */
export function verifyDir(dir, { pubkey, version } = {}) {
  const problems = [];
  const names = readdirSync(dir);
  const sigs = names.filter(n => n.endsWith('.sig'));
  if (sigs.length === 0) return [`no .sig files in ${dir}`];

  const work = mkdtempSync(join(tmpdir(), 'maquette-verify-'));
  try {
    const keyFile = join(work, 'updater.pub');
    writeFileSync(keyFile, decode(pubkey));
    for (const sig of sigs) {
      const file = join(dir, sig.slice(0, -'.sig'.length));
      if (!existsSync(file)) {
        problems.push(`${sig}: the file it signs is missing`);
        continue;
      }
      const sigFile = join(work, `${basename(file)}.minisig`);
      writeFileSync(sigFile, decode(readFileSync(join(dir, sig), 'utf8')));
      const run = spawnSync('minisign', ['-V', '-q', '-p', keyFile, '-m', file, '-x', sigFile], {
        encoding: 'utf8',
      });
      if (run.error) throw new Error(`Cannot run minisign: ${run.error.message}`);
      if (run.status !== 0) {
        problems.push(`${sig}: does not verify (${(run.stderr || run.stdout).trim()})`);
      } else {
        console.log(`ok    ${sig}`);
      }
    }
  } finally {
    rmSync(work, { recursive: true, force: true });
  }

  for (const [platform, matches] of Object.entries(PLATFORMS)) {
    if (!sigs.some(sig => matches(sig.slice(0, -'.sig'.length)))) {
      problems.push(`${platform}: no signed update file`);
    }
  }

  const manifestPath = join(dir, 'latest.json');
  if (existsSync(manifestPath)) problems.push(...checkManifest(dir, manifestPath, version));
  return problems;
}

/** latest.json names files in `dir`, carries their signatures and the right version. */
function checkManifest(dir, manifestPath, version) {
  const problems = [];
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  if (version !== undefined && manifest.version !== normalize(version)) {
    problems.push(`latest.json says version ${manifest.version}, expected ${normalize(version)}`);
  }
  for (const platform of Object.keys(PLATFORMS)) {
    const entry = manifest.platforms?.[platform];
    if (!entry) {
      problems.push(`latest.json: no ${platform} entry`);
      continue;
    }
    const name = decodeURIComponent(entry.url.split('/').pop() ?? '');
    const sigPath = join(dir, `${name}.sig`);
    if (!existsSync(join(dir, name)) || !existsSync(sigPath)) {
      problems.push(`latest.json: ${platform} points at ${name}, which is not in the release`);
    } else if (readFileSync(sigPath, 'utf8').trim() !== entry.signature) {
      problems.push(`latest.json: the ${platform} signature is not the one in ${name}.sig`);
    }
  }
  return problems;
}

function main() {
  const args = process.argv.slice(2);
  const dir = args[0];
  const at = args.indexOf('--version');
  const version = at === -1 ? undefined : args[at + 1];
  if (!dir || dir.startsWith('-')) {
    console.error('Usage: verify-signatures.mjs <dir> [--version X.Y.Z]');
    process.exitCode = 2;
    return;
  }
  if (!hasMinisign()) {
    console.error('minisign is not on the PATH (apt install minisign, brew install minisign).');
    process.exitCode = 1;
    return;
  }
  const problems = verifyDir(dir, { pubkey: shippedPubkey(), version });
  if (problems.length > 0) {
    console.error('Signature check failed:');
    for (const p of problems) console.error(`  - ${p}`);
    process.exitCode = 1;
    return;
  }
  console.log('Every signature verifies against the shipped public key.');
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
