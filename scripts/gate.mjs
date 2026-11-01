/**
 * The local gate, with the Node and Rust checks side by side.
 *
 *   pnpm gate                the whole gate: has to exit 0 before a pull request
 *   pnpm gate:fast           only what the changes since main touch (proves less)
 *
 * `cargo test -p maquette-desktop` writes the IPC bindings and tool-names.txt
 * while the Node lane reads them. It only writes a file that differs, and
 * atomically, so current files are never touched and a reader never sees half a
 * file. A file that did change leaves the Node lane's verdict unknown, since it
 * may have read either version, so the gate fails then and asks for another run.
 */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fastPlan, fullPlan } from './gate/plan.mjs';
import { runLanes, stopAll, summary } from './gate/run.mjs';

const root = resolve(import.meta.dirname, '..');
const GENERATED = [
  'apps/web/src/ipc/generated/bindings.ts',
  'apps/desktop/src-tauri/tool-names.txt',
];
const STATE_DIR = join(root, 'node_modules', '.cache', 'gate');
const INSTALL_NEXTEST = 'cargo install cargo-nextest --locked';

const sha = text => createHash('sha256').update(text).digest('hex');

/** The generated files as the next reader sees them, line endings aside (a Windows checkout has CRLF). */
function generatedState() {
  return GENERATED.map(file => {
    const path = join(root, file);
    return existsSync(path) ? sha(readFileSync(path, 'utf8').replace(/\r\n/g, '\n')) : 'missing';
  }).join();
}

/**
 * The cache folder for this lockfile. ESLint and Prettier do not re-check a file
 * when a plugin's code changes, so a dependency change starts a new cache.
 */
function cacheFolder() {
  const key = sha(readFileSync(join(root, 'pnpm-lock.yaml'))).slice(0, 12);
  const parent = join(root, 'node_modules', '.cache', 'gate-lint');
  if (existsSync(parent)) {
    for (const old of readdirSync(parent)) {
      if (old !== key) rmSync(join(parent, old), { recursive: true, force: true });
    }
  }
  return join(parent, key);
}

const hasNextest = () =>
  spawnSync('cargo', ['nextest', '--version'], { cwd: root, stdio: 'ignore' }).status === 0;

/** Lines of `git` output split on NUL, or null when the command fails. */
function gitPaths(...args) {
  const r = spawnSync('git', [...args, '-z'], { cwd: root, encoding: 'utf8' });
  return r.status === 0 ? r.stdout.split('\0').filter(Boolean) : null;
}

/** The commit this branch left main at, or null when there is no main to compare with. */
function forkPoint() {
  for (const main of ['origin/main', 'main']) {
    const r = spawnSync('git', ['merge-base', 'HEAD', main], { cwd: root, encoding: 'utf8' });
    if (r.status === 0) return r.stdout.trim();
  }
  return null;
}

/** Paths changed since `base`: commits, staged and unstaged edits, new untracked files. */
function changedSince(base) {
  const lists = [
    gitPaths('diff', '--name-only', '--no-renames', base),
    gitPaths('ls-files', '--others', '--exclude-standard'),
  ];
  return lists.includes(null) ? null : [...new Set(lists.flat())].sort();
}

/** The plan, plus the notices to repeat in the summary. */
function choosePlan({ fast }) {
  const nextest = hasNextest();
  const cacheDir = cacheFolder();
  const notices = [];
  if (!nextest) {
    notices.push(
      `NOTICE: cargo-nextest is not installed, so the Rust tests ran on cargo test: the same tests, slower. Install it with: ${INSTALL_NEXTEST}`
    );
  }
  if (!fast) return { plan: fullPlan({ nextest, cacheDir }), notices };

  notices.push(
    'NOTICE: gate:fast proves less than the whole gate. Run pnpm gate before a pull request.'
  );
  const base = forkPoint();
  const files = base && changedSince(base);
  if (!files) {
    notices.push('NOTICE: could not tell what changed since main, so this ran the whole gate.');
    return { plan: fullPlan({ nextest, cacheDir }), notices };
  }
  const exists = file => existsSync(join(root, file));
  return { plan: fastPlan(files, { nextest, cacheDir, base, exists }), notices };
}

const argv = process.argv.slice(2).filter(arg => arg !== '--');
const unknown = argv.filter(arg => arg !== '--fast');
if (unknown.length > 0) {
  console.error(`gate: unknown argument ${unknown.join(' ')}`);
  process.exit(2);
}

let stopping = false;
for (const [signal, code] of [
  ['SIGHUP', 129],
  ['SIGINT', 130],
  ['SIGTERM', 143],
]) {
  process.on(signal, () => {
    if (stopping) return;
    stopping = true;
    stopAll(() => process.exit(code));
  });
}

process.chdir(root);
const started = performance.now();
const { plan, notices } = choosePlan({ fast: argv.includes('--fast') });
notices.forEach(notice => console.log(notice));

const generatedBefore = generatedState();
const env = process.stdout.isTTY
  ? { ...process.env, FORCE_COLOR: '1', CARGO_TERM_COLOR: 'always' }
  : process.env;
const results = await runLanes(plan, { logDir: join(STATE_DIR, 'logs'), env });

if (results.length === 0) {
  console.log('gate: nothing changed since main, so nothing to check.');
  process.exit(0);
}

if (generatedState() !== generatedBefore) {
  results.push({
    lane: 'all',
    name: 'generated files',
    status: 'FAILED',
    seconds: 0,
    logFile: 'the diff',
    tail: [
      `The Rust tests rewrote what was stale of: ${GENERATED.join(', ')}.`,
      'The Node lane may have read either version, so its result does not count.',
      'Look at the diff, name any new tool in the docs, commit the files and run the gate again.',
    ],
  });
}

console.log(summary(results, (performance.now() - started) / 1000, notices));
process.exit(results.some(r => r.status === 'FAILED') ? 1 : 0);
