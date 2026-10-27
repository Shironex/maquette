/**
 * The title and notes of a release, from the file the tagged commit carries:
 * `docs/releases/vX.Y.Z.md`. A first line `# vX.Y.Z: short summary` becomes
 * the release title and the rest the notes. Without a heading the title is
 * `vX.Y.Z`; without the file both fall back to a short default.
 *
 *   node scripts/release/release-notes.mjs 0.2.0 --title title.txt --body notes.md
 *
 * The notes also go into latest.json, which the app shows in About.
 */
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { normalize } from './bump-version.mjs';

const ROOT = resolve(import.meta.dirname, '../..');

/** Where the notes of `version` live, relative to the repo root. */
export function notesPath(version) {
  return `docs/releases/v${normalize(version)}.md`;
}

/** `{ title, body, source }` for `version`; `source` is the file used, or `null`. */
export function readNotes(root, input) {
  const version = normalize(input);
  const rel = notesPath(version);
  const path = join(root, rel);
  if (!existsSync(path)) {
    return {
      title: `v${version}`,
      body:
        `Maquette ${version}. Download the installer for your system below: the \`.dmg\` for ` +
        'macOS on Apple silicon, the `-setup.exe` for Windows.',
      source: null,
    };
  }
  const text = readFileSync(path, 'utf8').replace(/\r\n/g, '\n').trim();
  const heading = /^#\s+(.+)\n?/.exec(text);
  if (!heading) return { title: `v${version}`, body: text, source: rel };
  return { title: heading[1].trim(), body: text.slice(heading[0].length).trim(), source: rel };
}

function main() {
  const args = process.argv.slice(2);
  const flag = name => {
    const at = args.indexOf(name);
    return at === -1 ? null : args[at + 1];
  };
  const version = args[0];
  const titleFile = flag('--title');
  const bodyFile = flag('--body');
  if (!version || version.startsWith('-') || !titleFile || !bodyFile) {
    console.error('Usage: release-notes.mjs <version> --title <file> --body <file>');
    process.exitCode = 2;
    return;
  }
  const { title, body, source } = readNotes(ROOT, version);
  writeFileSync(titleFile, `${title}\n`);
  writeFileSync(bodyFile, `${body}\n`);
  console.log(
    source ? `Notes from ${source}: "${title}".` : `No ${notesPath(version)}, using the default.`
  );
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
