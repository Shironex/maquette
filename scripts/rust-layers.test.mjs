/** Tests for scripts/rust-layers.mjs: `pnpm test:scripts`. */
import assert from 'node:assert/strict';
import { describe, test } from 'node:test';

import { cratePaths, LAYER_MAP, upwardUses } from './rust-layers.mjs';

const MAP = {
  crate: 'src',
  layers: { base: [], model: ['base'], glb: ['base'], pipeline: ['model', 'glb'] },
  modules: {
    lib: 'pipeline',
    error: 'base',
    project: 'model',
    inbox: 'pipeline',
    'inbox::math': 'glb',
    pipeline: 'pipeline',
  },
};

/** An IMetaCtx over an in-memory file set. */
function ctx(files, map = MAP) {
  const all = { [LAYER_MAP]: JSON.stringify(map), ...files };
  return {
    read: rel => all[rel] ?? null,
    glob: pattern => {
      const dir = pattern.slice(0, pattern.indexOf('/**'));
      return Object.keys(all).filter(f => f.startsWith(`${dir}/`) && f.endsWith('.rs'));
    },
  };
}

describe('cratePaths', () => {
  test('reads plain, grouped, nested and inline paths', () => {
    const source = `
      use crate::error::CoreResult;
      use crate::{project, pipeline::{history::{self, LogLevel}, tools::Toolbox}};
      fn f() { crate::inbox::math::bounds(); }
    `;
    assert.deepEqual(
      cratePaths(source).map(p => p.join('::')),
      [
        'error::CoreResult',
        'project',
        'pipeline::history',
        'pipeline::history::LogLevel',
        'pipeline::tools::Toolbox',
        'inbox::math::bounds',
      ]
    );
  });

  test('skips paths in comments and doc comments', () => {
    const source = '//! See [`crate::pipeline`].\n/* crate::inbox */\nuse crate::error;\n';
    assert.deepEqual(
      cratePaths(source).map(p => p.join('::')),
      ['error']
    );
  });
});

describe('upwardUses', () => {
  test('counts uses of higher layers per file and module, and allows lower ones', () => {
    const uses = upwardUses(
      ctx({
        'src/lib.rs': 'pub mod project;',
        'src/project.rs':
          'use crate::error;\nuse crate::pipeline::run;\nfn f() { crate::pipeline::go(); }',
        'src/pipeline/mod.rs': 'use crate::{project, error, inbox::math};',
      })
    );
    assert.deepEqual(uses, { 'src/project.rs -> pipeline': 2 });
  });

  test('the longest module path wins for the file and for the path it uses', () => {
    const uses = upwardUses(
      ctx({
        'src/inbox/math.rs': 'use crate::inbox::checks;\nuse crate::error;',
        'src/project.rs': 'use crate::inbox::math::bounds;',
      })
    );
    assert.deepEqual(uses, {
      'src/inbox/math.rs -> inbox': 1,
      'src/project.rs -> inbox::math': 1,
    });
  });

  test('a root re-export counts as the module lib.rs takes it from', () => {
    const uses = upwardUses(
      ctx({
        'src/lib.rs': 'pub use pipeline::{Runner, RunState};\npub use error::CoreError;',
        'src/project.rs': 'use crate::{CoreError, Runner};',
      })
    );
    assert.deepEqual(uses, { 'src/project.rs -> pipeline': 1 });
  });

  test('a module kept at its old path by a pub use of another crate keeps its layer', () => {
    const uses = upwardUses(
      ctx({
        'src/lib.rs': 'pub use other_base::error;\npub mod project;',
        'src/project.rs': 'use crate::error::CoreResult;',
        'src/inbox/math.rs': 'use crate::error;\nuse crate::project;',
      })
    );
    assert.deepEqual(uses, { 'src/inbox/math.rs -> project': 1 });
  });

  test('a module or a used path missing from the map fails', () => {
    assert.throws(() => upwardUses(ctx({ 'src/thumbs.rs': '' })), /module thumbs is not in/);
    assert.throws(
      () => upwardUses(ctx({ 'src/project.rs': 'use crate::thumbs;' })),
      /crate::thumbs is not in/
    );
    assert.throws(
      () => upwardUses(ctx({}, { ...MAP, modules: { lib: 'nowhere' } })),
      /unknown layer "nowhere"/
    );
  });
});
