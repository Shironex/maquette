/** Tests for scripts/crate-layers.mjs: `pnpm test:scripts`. */
import assert from 'node:assert/strict';
import { describe, test } from 'node:test';

import { CRATE_MAP, crateLayers, crateViolations, METADATA } from './crate-layers.mjs';

const ALLOWED = {
  'maquette-base': [],
  'maquette-preview': ['maquette-base'],
  'maquette-core': ['maquette-base', 'maquette-preview'],
};

/** `cargo metadata` output for crates given as name to dependency names. */
function metadata(crates) {
  return {
    packages: Object.entries(crates).map(([name, deps]) => ({
      name,
      dependencies: deps.map(d => (typeof d === 'string' ? { name: d, kind: null } : d)),
    })),
  };
}

describe('crateViolations', () => {
  test('allows the listed workspace crates and ignores outside ones', () => {
    const meta = metadata({
      'maquette-base': ['serde'],
      'maquette-preview': ['maquette-base', 'hyper'],
      'maquette-core': ['maquette-base', 'maquette-preview', 'serde'],
    });
    assert.deepEqual(crateViolations(meta, ALLOWED), []);
  });

  test('a dependency that points up fails, dev dependencies included', () => {
    const meta = metadata({
      'maquette-base': [{ name: 'maquette-core', kind: 'dev' }],
      'maquette-preview': ['maquette-base', 'maquette-core'],
      'maquette-core': ['maquette-base', 'maquette-preview'],
    });
    assert.deepEqual(
      crateViolations(meta, ALLOWED).map(v => v.crate),
      ['maquette-base', 'maquette-preview']
    );
  });

  test('a crate missing from the map and a map entry with no crate both fail', () => {
    const meta = metadata({ 'maquette-base': [], 'maquette-lab': [] });
    const messages = crateViolations(meta, {
      'maquette-base': [],
      'maquette-gone': [],
    }).map(v => v.message);
    assert.equal(messages.length, 2);
    assert.match(messages[0], /maquette-lab is not in/);
    assert.match(messages[1], /names maquette-gone, which is not a workspace crate/);
  });
});

describe('crateLayers', () => {
  test('reads the map and runs cargo metadata', () => {
    const ran = [];
    const ctx = {
      read: rel => (rel === CRATE_MAP ? JSON.stringify(ALLOWED) : null),
      exec: cmd => {
        ran.push(cmd);
        const meta = metadata({
          'maquette-base': ['maquette-preview'],
          'maquette-preview': ['maquette-base'],
          'maquette-core': ['maquette-base'],
        });
        return { code: 0, stdout: JSON.stringify(meta), stderr: '' };
      },
    };
    assert.equal(crateLayers(ctx).length, 1);
    assert.deepEqual(ran, [METADATA]);
  });

  test('a missing map or a failing cargo throws', () => {
    assert.throws(() => crateLayers({ read: () => null }), /is missing/);
    const ctx = {
      read: () => '{}',
      exec: () => ({ code: 101, stdout: '', stderr: 'no cargo' }),
    };
    assert.throws(() => crateLayers(ctx), /failed: no cargo/);
  });
});
