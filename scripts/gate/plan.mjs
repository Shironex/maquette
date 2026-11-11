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

/**
 * Files one part of the repo reads from another's folder, by a path or from disk,
 * so the changed paths alone do not show who is affected. These run the whole gate.
 * A test scans .ts, .tsx, .mjs, .astro and .rs under apps, packages and crates for
 * `../` paths that leave their app, and needs each one covered. It does not see
 * .css, .json, .js, .mts or .cjs files, anything in scripts/, or a path that does
 * not start with `../`, such as one built with join(). Those are listed by hand.
 */
const WHOLE_GATE = [
  // Vitest (state/bridge-shim.test.ts) and the mock bundle read the Rust crate's shim.
  'crates/maquette-preview/src/frame/shim.js',
  // Vitest (state/plan-text.test.ts) keeps the view's trim set equal to this one.
  'crates/maquette-model/src/plans/text.rs',
  // Vitest (ipc/mock/plan-presets.test.ts) keeps the mock's starter notes equal to these.
  'crates/maquette-model/src/plans/presets/seeds.json',
  // Vitest (ipc/mock/plan-overlap.test.ts) runs the mock's overlap check on these cases.
  'crates/maquette-model/src/plans/overlap/cases.json',
  // Vitest (features/viewer/recipe-layout.test.ts) composes these placements as the core does.
  'crates/maquette-model/src/layout/placement_cases.json',
  // Vitest (state/recipe-timeline.test.ts) plays these timeline cases as the core does.
  'crates/maquette-model/src/recipes/fixtures/timeline.json',
  // Vitest (state/recipe-shares.test.ts) moves these change spans as the core does.
  'crates/maquette-model/src/recipes/fixtures/clip_moves.json',
  // Vitest (state/recipe-pins.test.ts) holds these pins as the core does.
  'crates/maquette-model/src/recipes/fixtures/pins.json',
  // Vitest (state/recipe-keys.test.ts) blends these pose keys as the core does.
  'crates/maquette-model/src/recipes/fixtures/pose_keys.json',
  // Vitest (state/recipe-grips.test.ts) lays these grips as the core does.
  'crates/maquette-model/src/recipes/fixtures/grips.json',
  // The site's copy and docs read the app's version; so do the release scripts.
  'apps/desktop/src-tauri/tauri.conf.json',
  // Written by the Rust tests; the script tests, the meta lint and Vitest
  // (ipc/mock/mock-tool-groups.test.ts) read it.
  'apps/desktop/src-tauri/tool-names.txt',
  // Vitest (ipc/mock/mock-tool-groups.test.ts) keeps the mock's tool groups equal to these.
  'apps/desktop/src-tauri/src/mcp/tools/groups.rs',
  // Written by the Rust tests; typecheck, Vitest and the app read it.
  'apps/web/src/ipc/generated/bindings.ts',
  // Vitest (state/about.test.ts) reads this form, which otherwise counts as docs.
  '.github/ISSUE_TEMPLATE/bug_report.yml',
  // The Rust tests (thumbs) hold the preview rig's tone mapping and exposure to the site scene's.
  'apps/site/src/scene/Workbench.tsx',
];
/**
 * Folders of the same kind: maquette-glb's tests read the mock models, and the
 * site's build imports the web app's Vite plugin.
 */
const WHOLE_GATE_FOLDERS = ['apps/web/src/assets/mock/', 'apps/web/vite/'];
/** A package's own config (package.json, vite, tsconfig), which reaches past its files. */
const PACKAGE_CONFIG = /^(apps|packages)\/[^/]+\/(?![^/]+\.md$)[^/]+$/;
/** The skills' scripts, which core compiles in and its tests compare. */
const SKILL_SCRIPTS = /^plugins\/.*\/scripts\//;

const readsAcrossParts = file =>
  WHOLE_GATE.includes(file) ||
  WHOLE_GATE_FOLDERS.some(folder => file.startsWith(folder)) ||
  PACKAGE_CONFIG.test(file);

/**
 * The longest list of paths handed to a tool. cmd.exe stops at about 8,000
 * characters, and a longer list would fail for no fault in the code.
 */
const MAX_PATH_CHARS = 6000;
const fitsCommandLine = paths => paths.join(' ').length <= MAX_PATH_CHARS;

const LINTABLE = /\.(ts|tsx|mts|cts|js|jsx|mjs|cjs)$/;
const NOT_LINTED = /(^|\/)generated\//;

const step = (name, cmd, ...args) => ({ name, cmd, args });
const pnpm = (name, ...args) => step(name, 'pnpm', ...args);
/**
 * A step that takes every core: only one of these runs at a time. Vitest beside
 * the Blender tests pushed the load past 45 on 8 cores, slowed both more than it
 * saved and timed out healthy tests.
 */
const heavy = s => ({ ...s, exclusive: true });

/**
 * The cache flags for ESLint or Prettier, under node_modules/.cache. Hashes,
 * not file times, so switching branches keeps what is still true.
 */
function cacheFlags(tool, cacheDir) {
  return ['--cache', '--cache-strategy', 'content', '--cache-location', join(cacheDir, tool)];
}

/** The Rust test step: nextest and the doc tests it does not run, or `cargo test` for both. */
function rustTests(nextest) {
  if (!nextest) return [heavy(pnpm('rust:test', 'rust:test'))];
  return [
    heavy(step('nextest', 'cargo', 'nextest', 'run', '--workspace')),
    step('doc tests', 'cargo', 'test', '--workspace', '--doc'),
  ];
}

const rustLane = nextest => [
  pnpm('rust:fmt', 'rust:fmt'),
  heavy(pnpm('rust:lint', 'rust:lint')),
  ...rustTests(nextest),
];

/**
 * The whole gate: the same steps in the same order per lane as ever. ESLint has
 * no cache here: rules read sibling files and package.json from disk, so a
 * cached pass can outlive a deleted required test. Prettier judges a file alone.
 */
export function fullPlan({ nextest, cacheDir }) {
  return {
    node: [
      pnpm('lint', 'lint'),
      pnpm('lint:meta', 'lint:meta'),
      pnpm('typecheck', 'typecheck'),
      heavy(pnpm('test', 'test')),
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
    if (readsAcrossParts(file)) touched.other = true;
    else if (SKILL_SCRIPTS.test(file)) {
      touched.skills = true;
      touched.rust = true;
    } else if (file.startsWith('apps/site/')) touched.site = true;
    else if (DOCS_RE.test(file)) touched.docs = true;
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
 * The fast gate for a list of changed paths. A path it cannot place, or one
 * another part of the repo reads, runs the whole gate, and no list at all runs
 * nothing.
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
  if (!fitsCommandLine(lintable)) {
    node.push(pnpm('lint', 'lint', ...cacheFlags('eslint', cacheDir)));
  } else if (lintable.length > 0) {
    const flags = cacheFlags('eslint', cacheDir);
    node.push(pnpm('lint', 'exec', 'eslint', '--no-warn-ignored', ...flags, '--', ...lintable));
  }
  // Whole-repo checks over the sources, the skills and the README: quick, and
  // no list of files can stand in for them.
  node.push(pnpm('lint:meta', 'lint:meta'));
  if (frontend) node.push(pnpm('typecheck', 'typecheck'));
  if (touched.web || touched.site) {
    // --changed follows imports only, and tests also read tokens, models and
    // fixtures from disk, so a change to anything but a module runs them all.
    const frontendFiles = files.filter(f => /^(apps\/(web|site)|packages)\//.test(f));
    const modulesOnly = frontendFiles.every(f => LINTABLE.test(f) || DOCS_RE.test(f));
    const inSiteDocs = frontendFiles.some(f => f.startsWith('apps/site/') && DOCS_RE.test(f));
    node.push(
      modulesOnly && !inSiteDocs
        ? heavy(pnpm('test', 'exec', 'vitest', 'run', '--changed', base, '--passWithNoTests'))
        : heavy(pnpm('test', 'test'))
    );
  }
  if (touched.scripts) node.push(pnpm('test:scripts', 'test:scripts'));
  if (touched.site || files.some(f => f.startsWith('packages/'))) {
    node.push(pnpm('build:site', 'build:site'));
  }
  if (!fitsCommandLine(present)) {
    node.push(pnpm('format:check', 'format:check', ...cacheFlags('prettier', cacheDir)));
  } else if (present.length > 0) {
    const flags = cacheFlags('prettier', cacheDir);
    node.push(
      pnpm(
        'format:check',
        'exec',
        'prettier',
        '--check',
        '--ignore-unknown',
        ...flags,
        '--',
        ...present
      )
    );
  }
  return { node, rust: touched.rust ? rustLane(nextest) : [] };
}
