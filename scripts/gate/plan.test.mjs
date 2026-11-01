/** Tests for scripts/gate/plan.mjs: `pnpm test:scripts`. */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, test } from 'node:test';

import { classify, DOCS, fastPlan, fullPlan } from './plan.mjs';

const root = resolve(import.meta.dirname, '../..');
const options = { nextest: true, cacheDir: 'cache', base: 'abc', exists: () => true };
const names = steps => steps.map(s => s.name);
const lanes = plan => ({ node: names(plan.node), rust: names(plan.rust) });

describe('fullPlan', () => {
  test('runs every step of the gate, in order per lane', () => {
    assert.deepEqual(lanes(fullPlan(options)), {
      node: [
        'lint',
        'lint:meta',
        'typecheck',
        'test',
        'test:scripts',
        'build:site',
        'format:check',
      ],
      rust: ['rust:fmt', 'rust:lint', 'nextest', 'doc tests'],
    });
  });

  test('the steps that take every core take turns', () => {
    const { node, rust } = fullPlan(options);
    const heavy = [...node, ...rust].filter(s => s.exclusive);
    assert.deepEqual(names(heavy), ['test', 'rust:lint', 'nextest']);
  });

  test('every pnpm step names a script package.json has', () => {
    const { scripts } = JSON.parse(readFileSync(resolve(root, 'package.json'), 'utf8'));
    const { node, rust } = fullPlan(options);
    for (const step of [...node, ...rust].filter(s => s.cmd === 'pnpm')) {
      assert.ok(scripts[step.args[0]], `${step.args[0]} is a script`);
    }
  });

  test('without nextest the Rust tests run on cargo test, which has the doc tests too', () => {
    assert.deepEqual(names(fullPlan({ ...options, nextest: false }).rust), [
      'rust:fmt',
      'rust:lint',
      'rust:test',
    ]);
  });

  test('without a cache folder ESLint and Prettier are the plain scripts', () => {
    const { node } = fullPlan({ ...options, cacheDir: null });
    assert.deepEqual(node[0].args, ['lint']);
    assert.deepEqual(node.at(-1).args, ['format:check']);
  });

  test('the cache flags reach ESLint and Prettier and nothing else', () => {
    const { node, rust } = fullPlan(options);
    const cached = [...node, ...rust].filter(s => s.args.includes('--cache'));
    assert.deepEqual(names(cached), ['lint', 'format:check']);
  });
});

describe('classify', () => {
  test('keeps the docs pattern CI uses', () => {
    const ci = readFileSync(resolve(root, '.github/workflows/ci.yml'), 'utf8');
    const quoted = ci.match(/docs='(.+)'/);
    assert.equal(quoted?.[1], DOCS.replaceAll('\\\\', '\\'));
  });

  test('sorts the parts of the repo', () => {
    const only = file => Object.keys(classify([file])).filter(k => classify([file])[k]);
    assert.deepEqual(only('docs/architecture.md'), ['docs']);
    assert.deepEqual(only('README.md'), ['docs']);
    assert.deepEqual(only('apps/site/src/content/docs/index.md'), ['site']);
    assert.deepEqual(only('apps/web/src/App.tsx'), ['web']);
    assert.deepEqual(only('packages/ui/src/index.ts'), ['web']);
    assert.deepEqual(only('scripts/gate.mjs'), ['scripts']);
    assert.deepEqual(only('crates/maquette-core/src/lib.rs'), ['rust']);
    assert.deepEqual(only('apps/desktop/src-tauri/tool-names.txt'), ['rust']);
    assert.deepEqual(only('plugins/maquette/skills/a/SKILL.md'), ['docs']);
    assert.deepEqual(only('plugins/maquette/.mcp.json'), ['skills']);
  });

  test('anything it cannot place is unsure', () => {
    for (const file of [
      'package.json',
      'Cargo.lock',
      'pnpm-lock.yaml',
      '.github/workflows/ci.yml',
    ]) {
      assert.equal(classify([file]).other, true, file);
    }
  });

  test('the generated bindings need both lanes', () => {
    const touched = classify(['apps/web/src/ipc/generated/bindings.ts']);
    assert.ok(touched.web && touched.rust);
  });
});

describe('fastPlan', () => {
  test('no changes run nothing', () => {
    assert.deepEqual(lanes(fastPlan([], options)), { node: [], rust: [] });
  });

  test('docs only: the meta lint and Prettier on the files', () => {
    const plan = fastPlan(['docs/a.md'], options);
    assert.deepEqual(lanes(plan), { node: ['lint:meta', 'format:check'], rust: [] });
    assert.ok(plan.node[1].args.includes('docs/a.md'));
  });

  test('a web change skips Rust, the scripts tests and the site', () => {
    const plan = fastPlan(['apps/web/src/App.tsx'], options);
    assert.deepEqual(lanes(plan), {
      node: ['lint', 'lint:meta', 'typecheck', 'test', 'format:check'],
      rust: [],
    });
    assert.ok(plan.node[0].args.includes('apps/web/src/App.tsx'));
    assert.deepEqual(plan.node[3].args.slice(1, 5), ['vitest', 'run', '--changed', 'abc']);
  });

  test('a UI kit or site change builds the site', () => {
    for (const file of ['packages/ui/src/index.ts', 'apps/site/src/a.ts']) {
      assert.ok(names(fastPlan([file], options).node).includes('build:site'), file);
    }
  });

  test('a Rust change runs the Rust lane and the meta lint', () => {
    assert.deepEqual(lanes(fastPlan(['crates/maquette-base/src/fs.rs'], options)), {
      node: ['lint:meta', 'format:check'],
      rust: ['rust:fmt', 'rust:lint', 'nextest', 'doc tests'],
    });
  });

  test('a script change runs the scripts tests', () => {
    const { node } = fastPlan(['scripts/gate.mjs'], options);
    assert.deepEqual(names(node), [
      'lint',
      'lint:meta',
      'typecheck',
      'test:scripts',
      'format:check',
    ]);
  });

  test('a path it cannot place runs the whole gate', () => {
    assert.deepEqual(
      lanes(fastPlan(['apps/web/src/App.tsx', 'package.json'], options)),
      lanes(fullPlan(options))
    );
  });

  test('a removed file is not handed to the tools but still counts', () => {
    const plan = fastPlan(['apps/web/src/Gone.tsx'], { ...options, exists: () => false });
    assert.deepEqual(names(plan.node), ['lint:meta', 'typecheck', 'test']);
  });

  test('a long list goes to the tools as the whole repo', () => {
    const files = Array.from({ length: 201 }, (_, i) => `apps/web/src/f${i}.ts`);
    const plan = fastPlan(files, options);
    assert.equal(plan.node[0].args[0], 'lint');
    assert.equal(plan.node.at(-1).args[0], 'format:check');
  });
});
