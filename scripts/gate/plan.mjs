/**
 * What `pnpm gate` and `pnpm gate:fast` run. Pure, so the tests can read it;
 * scripts/gate.mjs does the running.
 */
import { join } from 'node:path';

/**
 * A change that only touches these is docs to CI (the `changes` job in
 * .github/workflows/ci.yml, which keeps the same pattern; a test holds them
 * equal). apps/site is left out first: its Markdown is built, so it is code.
 */
export const DOCS =
  '^(docs/|assets/|\\.github/ISSUE_TEMPLATE/|LICENSE$|\\.github/[^/]+\\.(md|yml)$|\\.github/CODEOWNERS$)|^([^/]+/)*[^/]+\\.md$';
const DOCS_RE = new RegExp(DOCS);

/** More paths than this go to the tool as the whole repo, to stay under Windows' command line limit. */
const MAX_FILE_ARGS = 200;

const LINTABLE = /\.(ts|tsx|mts|cts|js|jsx|mjs|cjs)$/;
const NOT_LINTED = /(^|\/)generated\//;

const step = (name, cmd, ...args) => ({ name, cmd, args });
const pnpm = (name, ...args) => step(name, 'pnpm', ...args);

/**
 * The cache flags for ESLint or Prettier, under node_modules/.cache. Hashes,
 * not file times, so switching branches keeps what is still true.
 */
function cacheFlags(tool, cacheDir) {
  if (!cacheDir) return [];
  return ['--cache', '--cache-strategy', 'content', '--cache-location', join(cacheDir, tool)];
}

/** The Rust test step: nextest and the doc tests it does not run, or `cargo test` for both. */
function rustTests(nextest) {
  if (!nextest) return [pnpm('rust:test', 'rust:test')];
  return [
    step('nextest', 'cargo', 'nextest', 'run', '--workspace'),
    step('doc tests', 'cargo', 'test', '--workspace', '--doc'),
  ];
}

const rustLane = nextest => [
  pnpm('rust:fmt', 'rust:fmt'),
  pnpm('rust:lint', 'rust:lint'),
  ...rustTests(nextest),
];

/** The whole gate: the same steps in the same order per lane as ever. */
export function fullPlan({ nextest, cacheDir }) {
  return {
    node: [
      pnpm('lint', 'lint', ...cacheFlags('eslint', cacheDir)),
      pnpm('lint:meta', 'lint:meta'),
      pnpm('typecheck', 'typecheck'),
      pnpm('test', 'test'),
      pnpm('test:scripts', 'test:scripts'),
      pnpm('build:site', 'build:site'),
      pnpm('format:check', 'format:check', ...cacheFlags('prettier', cacheDir)),
    ],
    rust: rustLane(nextest),
  };
}

/** Which parts of the repo a list of changed paths touches. `other` means unsure. */
export function classify(files) {
  const touched = {
    docs: false,
    web: false,
    site: false,
    scripts: false,
    rust: false,
    skills: false,
    other: false,
  };
  for (const file of files) {
    if (file.startsWith('apps/site/')) touched.site = true;
    else if (file === 'apps/web/src/ipc/generated/bindings.ts') {
      touched.web = true;
      touched.rust = true;
    } else if (DOCS_RE.test(file)) touched.docs = true;
    else if (file.startsWith('apps/web/') || file.startsWith('packages/')) touched.web = true;
    else if (file.startsWith('scripts/')) touched.scripts = true;
    else if (file.startsWith('plugins/') || file.startsWith('.claude-plugin/'))
      touched.skills = true;
    else if (file.startsWith('crates/') || file.startsWith('apps/desktop/src-tauri/'))
      touched.rust = true;
    else touched.other = true;
  }
  return touched;
}

/**
 * The fast gate for a list of changed paths. A path it cannot place runs the
 * whole gate, and no list at all runs nothing.
 *
 * `exists` says whether a path is still there: a removed file has nothing to
 * lint, but it still counts towards what changed.
 */
export function fastPlan(files, { nextest, cacheDir, base, exists }) {
  if (files.length === 0) return { node: [], rust: [] };
  const touched = classify(files);
  if (touched.other) return fullPlan({ nextest, cacheDir });

  const present = files.filter(exists);
  const lintable = present.filter(f => LINTABLE.test(f) && !NOT_LINTED.test(f));
  const frontend = touched.web || touched.site || touched.scripts;
  const node = [];
  if (lintable.length > MAX_FILE_ARGS) {
    node.push(pnpm('lint', 'lint', ...cacheFlags('eslint', cacheDir)));
  } else if (lintable.length > 0) {
    const flags = cacheFlags('eslint', cacheDir);
    node.push(pnpm('lint', 'exec', 'eslint', '--no-warn-ignored', ...flags, ...lintable));
  }
  // Whole-repo checks over the sources, the skills and the README: quick, and
  // no list of files can stand in for them.
  node.push(pnpm('lint:meta', 'lint:meta'));
  if (frontend) node.push(pnpm('typecheck', 'typecheck'));
  if (touched.web || touched.site) {
    node.push(pnpm('test', 'exec', 'vitest', 'run', '--changed', base, '--passWithNoTests'));
  }
  if (touched.scripts) node.push(pnpm('test:scripts', 'test:scripts'));
  if (touched.site || files.some(f => f.startsWith('packages/'))) {
    node.push(pnpm('build:site', 'build:site'));
  }
  if (present.length > MAX_FILE_ARGS) {
    node.push(pnpm('format:check', 'format:check', ...cacheFlags('prettier', cacheDir)));
  } else if (present.length > 0) {
    const flags = cacheFlags('prettier', cacheDir);
    node.push(
      pnpm('format:check', 'exec', 'prettier', '--check', '--ignore-unknown', ...flags, ...present)
    );
  }
  return { node, rust: touched.rust ? rustLane(nextest) : [] };
}
