/**
 * Sets or checks the app version in every file that carries it:
 *
 * - `package.json` (the root) and `apps/desktop/package.json`,
 * - `apps/desktop/src-tauri/tauri.conf.json`,
 * - `[workspace.package]` in the root `Cargo.toml` (every crate inherits it),
 * - and the workspace crates' entries in `Cargo.lock`, so the lock file never
 *   lags one build behind.
 *
 *   node scripts/release/bump-version.mjs 0.2.0           set it everywhere
 *   node scripts/release/bump-version.mjs --check         every file agrees
 *   node scripts/release/bump-version.mjs --check v0.2.0  every file says 0.2.0 (the tag check)
 *   node scripts/release/bump-version.mjs --print         print the version every file agrees on
 *
 * A leading `v` is accepted everywhere, so a tag can be passed as it is.
 * Files are edited in place as text, so their formatting and line endings
 * stay as they were. Runs the same on macOS and Windows.
 */
import { readdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(import.meta.dirname, '../..');

/** Semver with an optional pre-release, no build metadata. */
const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

/** The top-level `"version"` of a JSON file indented by two spaces. */
const JSON_VERSION = /^( {2}"version":\s*")([^"]*)(")/m;

const JSON_FILES = [
  'package.json',
  'apps/desktop/package.json',
  'apps/desktop/src-tauri/tauri.conf.json',
];

/** `0.2.0` from `v0.2.0` or `0.2.0`; throws on anything that is not semver. */
export function normalize(input) {
  const version = String(input ?? '')
    .trim()
    .replace(/^v/, '');
  if (!SEMVER.test(version)) {
    throw new Error(`Not a version: "${input}". Use X.Y.Z or X.Y.Z-pre.N, with or without a v.`);
  }
  return version;
}

/** The `version = "..."` line inside `[workspace.package]`, as a match on the whole text. */
function workspaceVersion(toml) {
  const header = /^\[workspace\.package\]\s*$/m.exec(toml);
  if (!header) return null;
  const start = header.index + header[0].length;
  const next = /^\[/m.exec(toml.slice(start));
  const end = next ? start + next.index : toml.length;
  const line = /^(version\s*=\s*")([^"]*)(")/m.exec(toml.slice(start, end));
  if (!line) return null;
  return { index: start + line.index, match: line };
}

/** The names of the workspace crates that inherit the workspace version. */
export function workspaceCrates(root) {
  const manifests = [];
  const crates = join(root, 'crates');
  if (existsSync(crates)) {
    for (const name of readdirSync(crates)) manifests.push(join(crates, name, 'Cargo.toml'));
  }
  manifests.push(join(root, 'apps/desktop/src-tauri/Cargo.toml'));
  return manifests
    .filter(file => existsSync(file))
    .map(file => readFileSync(file, 'utf8'))
    .filter(text => /^version\.workspace\s*=\s*true/m.test(text))
    .map(text => /^name\s*=\s*"([^"]+)"/m.exec(text)?.[1])
    .filter(Boolean);
}

/** The `version = "..."` line of `name` in Cargo.lock, as a regex over the whole file. */
function lockEntry(name) {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`(^name = "${escaped}"\\r?\\nversion = ")([^"]*)(")`, 'm');
}

/** Every place the version lives, with what it says today (`null` when it cannot be read). */
export function readVersions(root = ROOT) {
  const found = [];
  for (const file of JSON_FILES) {
    const text = readFileSync(join(root, file), 'utf8');
    found.push({ file, version: JSON_VERSION.exec(text)?.[2] ?? null });
  }
  const toml = readFileSync(join(root, 'Cargo.toml'), 'utf8');
  found.push({
    file: 'Cargo.toml [workspace.package]',
    version: workspaceVersion(toml)?.match[2] ?? null,
  });
  const lockPath = join(root, 'Cargo.lock');
  if (existsSync(lockPath)) {
    const lock = readFileSync(lockPath, 'utf8');
    for (const name of workspaceCrates(root)) {
      found.push({
        file: `Cargo.lock (${name})`,
        version: lockEntry(name).exec(lock)?.[2] ?? null,
      });
    }
  }
  return found;
}

/**
 * Checks that every file carries one version, and that it is `expected` when
 * given. Returns the version and a list of problems (empty when all is well).
 */
export function checkVersions(root = ROOT, expected) {
  const want = expected === undefined ? undefined : normalize(expected);
  const found = readVersions(root);
  const problems = [];
  for (const { file, version } of found) {
    if (version === null) problems.push(`${file}: no version found`);
  }
  const versions = [...new Set(found.map(f => f.version).filter(v => v !== null))];
  if (versions.length > 1) {
    problems.push(
      `the files disagree: ${found.map(f => `${f.file} = ${f.version ?? 'none'}`).join(', ')}`
    );
  }
  if (want !== undefined && versions.length === 1 && versions[0] !== want) {
    problems.push(`the files say ${versions[0]}, expected ${want}`);
  }
  return { version: versions.length === 1 ? versions[0] : null, problems, found };
}

function replaceOnce(text, regex, version, what) {
  if (!regex.test(text)) throw new Error(`No version to replace in ${what}`);
  return text.replace(regex, (_, before, _old, after) => `${before}${version}${after}`);
}

/** Writes `version` into every file. Returns the files it changed. */
export function setVersion(root = ROOT, input) {
  const version = normalize(input);
  const changed = [];
  const write = (file, next) => {
    const path = join(root, file);
    if (readFileSync(path, 'utf8') !== next) {
      writeFileSync(path, next);
      changed.push(file);
    }
  };

  for (const file of JSON_FILES) {
    write(file, replaceOnce(readFileSync(join(root, file), 'utf8'), JSON_VERSION, version, file));
  }

  const toml = readFileSync(join(root, 'Cargo.toml'), 'utf8');
  const entry = workspaceVersion(toml);
  if (!entry) throw new Error('No version under [workspace.package] in Cargo.toml');
  const [line, before, , after] = entry.match;
  write(
    'Cargo.toml',
    toml.slice(0, entry.index) + before + version + after + toml.slice(entry.index + line.length)
  );

  if (existsSync(join(root, 'Cargo.lock'))) {
    let lock = readFileSync(join(root, 'Cargo.lock'), 'utf8');
    for (const name of workspaceCrates(root)) {
      lock = replaceOnce(lock, lockEntry(name), version, `Cargo.lock (${name})`);
    }
    write('Cargo.lock', lock);
  }
  return changed;
}

function main() {
  const args = process.argv.slice(2);
  if (args[0] === '--check') {
    const { version, problems } = checkVersions(ROOT, args[1]);
    if (problems.length > 0) {
      console.error(`Version check failed${args[1] ? ` for ${args[1]}` : ''}:`);
      for (const p of problems) console.error(`  - ${p}`);
      console.error('Fix it with: node scripts/release/bump-version.mjs <version>');
      process.exitCode = 1;
      return;
    }
    console.log(`All version files say ${version}.`);
    return;
  }
  if (args[0] === '--print') {
    const { version, problems } = checkVersions(ROOT);
    if (problems.length > 0) {
      for (const p of problems) console.error(p);
      process.exitCode = 1;
      return;
    }
    console.log(version);
    return;
  }
  if (args.length !== 1 || args[0].startsWith('-')) {
    console.error('Usage: bump-version.mjs <version> | --check [version] | --print');
    process.exitCode = 2;
    return;
  }
  const changed = setVersion(ROOT, args[0]);
  const version = normalize(args[0]);
  console.log(
    changed.length > 0
      ? `Set ${version} in ${changed.join(', ')}.`
      : `Every file already says ${version}.`
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
