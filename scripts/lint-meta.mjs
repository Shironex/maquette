/**
 * Whole-repo checks ESLint and clippy cannot make one file at a time:
 *
 * - every TypeScript module in the app and the UI kit stays at or under 400
 *   lines (tests excluded);
 * - every Rust module stays at or under 400 code lines (comments and blank
 *   lines excluded, `tests.rs` test modules excluded);
 * - no ESLint rule resolves to `warn`, including what a spread preset sets;
 * - no inline `eslint-disable` or `@ts-ignore`: exceptions go in a scoped
 *   override in eslint.config.mjs, with the reason next to it.
 *
 *   pnpm lint:meta             check, exit 1 on any violation
 *   pnpm lint:meta --baseline  rewrite the baselines from today's offenders
 *
 * A baseline in scripts/lint-meta/baselines freezes today's offenders: they
 * may shrink, never grow, and an entry that is no longer needed fails until it
 * is removed. New offenders always fail.
 */
import { spawnSync } from 'node:child_process';
import {
  existsSync,
  globSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { join, resolve } from 'node:path';
import { isGrandfathered, loadBaseline, serializeBaseline } from '@noctcore/harness';
import { createFileSizeRatchetRule } from '@noctcore/lint-meta-rules';
import { createEslintConfigNoWarnRule } from '@noctcore/lint-meta-rules/resolved-config';

const root = resolve(import.meta.dirname, '..');
const BASELINE_DIR = 'scripts/lint-meta/baselines';
const CAP = 400;

/** The IMetaCtx the rules read the repo through. */
const ctx = {
  root,
  read(rel) {
    const abs = join(root, rel);
    return existsSync(abs) ? readFileSync(abs, 'utf8').replace(/\r\n/g, '\n') : null;
  },
  exists(rel) {
    return existsSync(join(root, rel));
  },
  glob(pattern) {
    // globSync also returns directories; every rule here wants files.
    return globSync(pattern, { cwd: root, exclude: p => p.includes('node_modules') })
      .map(p => p.replaceAll('\\', '/'))
      .filter(p => statSync(join(root, p)).isFile());
  },
  exec(cmd) {
    const r = spawnSync(cmd, { cwd: root, shell: true, encoding: 'utf8' });
    return { code: r.status ?? 1, stdout: r.stdout ?? '', stderr: r.stderr ?? '' };
  },
};

/** Lines that carry Rust code: no blank lines, no line or block comments. */
function rustCodeLines(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .split('\n')
    .filter(line => {
      const t = line.trim();
      return t !== '' && !t.startsWith('//');
    }).length;
}

/**
 * A ratchet over a per-file metric: files over `limit` fail unless the
 * baseline freezes them at a value they have not grown past.
 */
function ratchetRule({ id, description, files, measure, limit, message }) {
  const offenders = () => {
    const map = {};
    for (const file of files()) {
      const n = measure(ctx.read(file) ?? '');
      if (n > limit) map[file] = n;
    }
    return map;
  };
  return {
    id,
    category: 'source-text',
    ciCritical: true,
    description,
    baseline: offenders,
    run() {
      const baseline = loadBaseline(ctx, id, BASELINE_DIR);
      const current = offenders();
      const violations = [];
      for (const [file, n] of Object.entries(current)) {
        if (!isGrandfathered(baseline, file, n)) {
          violations.push({ file, rule: id, message: message(n) });
        }
      }
      for (const file of Object.keys(baseline)) {
        if (!(file in current)) {
          violations.push({
            file,
            rule: id,
            message: 'baseline entry is no longer needed. Remove it (pnpm lint:meta --baseline).',
          });
        }
      }
      return violations;
    },
  };
}

const rustFiles = () =>
  [
    ...ctx.glob('crates/*/src/**/*.rs'),
    ...ctx.glob('crates/*/examples/**/*.rs'),
    ...ctx.glob('apps/desktop/src-tauri/src/**/*.rs'),
  ]
    .filter(f => !f.endsWith('/tests.rs') && !f.includes('/tests/'))
    .sort();

const SOURCE_ROOTS = ['apps/web/src', 'packages/ui/src'];
const sourceFiles = () =>
  SOURCE_ROOTS.flatMap(r => [...ctx.glob(`${r}/**/*.ts`), ...ctx.glob(`${r}/**/*.tsx`)])
    .filter(f => !f.includes('/generated/') && f !== 'packages/ui/src/icons/paths.ts')
    .sort();

const INLINE_SUPPRESSION = /eslint-disable|@ts-ignore|@ts-nocheck|@ts-expect-error/g;

const rules = [
  createFileSizeRatchetRule({
    id: 'web-file-size',
    roots: ['apps/web/src'],
    cap: CAP,
    excludePrefixes: ['apps/web/src/ipc/generated/'],
    baselineDir: BASELINE_DIR,
  }),
  createFileSizeRatchetRule({
    id: 'ui-file-size',
    roots: ['packages/ui/src'],
    cap: CAP,
    // Generated from @phosphor-icons/core by scripts/gen-icons.mjs.
    excludePrefixes: ['packages/ui/src/icons/paths.ts'],
    baselineDir: BASELINE_DIR,
  }),
  ratchetRule({
    id: 'rust-module-size',
    description: `Rust modules stay at or under ${CAP} code lines (comments and blank lines excluded).`,
    files: rustFiles,
    measure: rustCodeLines,
    limit: CAP,
    message: n =>
      `${n} code lines exceeds the ${CAP} line cap. The module has more than one job: split it into submodules at its natural seams.`,
  }),
  ratchetRule({
    id: 'no-inline-suppression',
    description:
      'No inline eslint-disable or @ts-ignore; exceptions are scoped overrides with a reason.',
    files: sourceFiles,
    measure: text => (text.match(INLINE_SUPPRESSION) ?? []).length,
    limit: 0,
    message: n =>
      `${n} inline suppression(s). Fix the code, or add a scoped override with a one line reason to eslint.config.mjs.`,
  }),
  createEslintConfigNoWarnRule({ packages: ['.'], probes: probeFiles() }),
];

/** One probe per file shape the config scopes blocks to. */
function probeFiles() {
  return [
    'apps/web/src/__probe__.ts',
    'apps/web/src/__probe__.tsx',
    'apps/web/src/__probe__.test.tsx',
    'apps/web/src/features/viewer/__Probe__/__Probe__.tsx',
    'apps/web/src/features/shell/__Probe__/__Probe__.tsx',
    'packages/ui/src/__probe__.tsx',
    'packages/ui/src/__probe__.test.tsx',
    'scripts/__probe__.mjs',
  ];
}

if (process.argv.includes('--baseline')) {
  mkdirSync(join(root, BASELINE_DIR), { recursive: true });
  for (const rule of rules) {
    if (!rule.baseline) continue;
    const map = rule.baseline(ctx);
    const file = join(root, BASELINE_DIR, `${rule.id}.json`);
    if (Object.keys(map).length === 0) {
      rmSync(file, { force: true });
      continue;
    }
    writeFileSync(file, serializeBaseline(map));
    console.log(`baseline ${rule.id}: ${Object.keys(map).length} entries`);
  }
}

let failed = 0;
for (const rule of rules) {
  let violations;
  try {
    violations = [...(rule.run?.(ctx) ?? []), ...((await rule.runAsync?.(ctx)) ?? [])];
  } catch (error) {
    // A rule that throws is a failure, never a silent pass.
    console.error(`${rule.id}: threw`, error);
    failed += 1;
    continue;
  }
  for (const v of violations) console.error(`${v.rule}  ${v.file}  ${v.message}`);
  failed += violations.length;
}

if (failed > 0) {
  console.error(`\nlint-meta: ${failed} violation(s)`);
  process.exit(1);
}
console.log(`lint-meta: ${rules.length} checks passed`);
