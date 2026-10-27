/**
 * Writes the updater manifest, `latest.json`, from the signed files of a
 * release. Built once, after every platform's build, so no two jobs race to
 * rewrite it.
 *
 *   node scripts/release/latest-json.mjs --dir release-assets --version 0.2.0 \
 *     --repo Shironex/maquette --notes-file notes.md --out release-assets/latest.json
 *
 * The URLs point at the release's own download links
 * (`https://github.com/<repo>/releases/download/v<version>/<file>`), which
 * answer once the release is published.
 */
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { normalize } from './bump-version.mjs';

/** The updater platforms a release must carry, and the file each one installs. */
export const PLATFORMS = {
  'darwin-aarch64': name => name.endsWith('_aarch64.app.tar.gz'),
  'windows-x86_64': name => name.endsWith('_x64-setup.exe'),
};

/**
 * The manifest for the files in `dir`. Every platform needs exactly one
 * matching file with its `.sig` next to it.
 */
export function buildManifest({ dir, version: input, repo, notes, pubDate = new Date() }) {
  const version = normalize(input);
  const names = readdirSync(dir);
  const platforms = {};
  for (const [platform, matches] of Object.entries(PLATFORMS)) {
    const found = names.filter(matches);
    if (found.length !== 1) {
      throw new Error(
        `${platform}: expected one update file in ${dir}, found ${found.length} (${found.join(', ') || 'none'})`
      );
    }
    const [name] = found;
    if (!names.includes(`${name}.sig`)) throw new Error(`${platform}: ${name}.sig is missing`);
    platforms[platform] = {
      signature: readFileSync(join(dir, `${name}.sig`), 'utf8').trim(),
      url: `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(name)}`,
    };
  }
  return { version, notes, pub_date: pubDate.toISOString(), platforms };
}

function main() {
  const args = process.argv.slice(2);
  const flag = name => {
    const at = args.indexOf(name);
    return at === -1 ? null : args[at + 1];
  };
  const dir = flag('--dir');
  const version = flag('--version');
  const repo = flag('--repo');
  const notesFile = flag('--notes-file');
  const out = flag('--out');
  if (!dir || !version || !repo || !notesFile || !out) {
    console.error(
      'Usage: latest-json.mjs --dir <dir> --version <v> --repo <owner/name> --notes-file <file> --out <file>'
    );
    process.exitCode = 2;
    return;
  }
  const notes = readFileSync(notesFile, 'utf8').trim();
  const manifest = buildManifest({ dir, version, repo, notes });
  writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`Wrote ${out} for ${Object.keys(manifest.platforms).join(', ')}.`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
