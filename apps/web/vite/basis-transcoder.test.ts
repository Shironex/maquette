import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { stripBundledTranscoder } from './basis-transcoder';

// The app's own install of three, the one Vite bundles.
const three = resolve(import.meta.dirname, '../node_modules/three');
const loader = resolve(three, 'examples/jsm/loaders/KTX2Loader.js');

describe('the single Basis transcoder', () => {
  it('blanks both default URLs in the loader three ships', () => {
    const out = stripBundledTranscoder(readFileSync(loader, 'utf8'));
    expect(out).not.toContain('../libs/basis/basis_transcoder');
    expect(out).toContain("const WASM_BIN_URL = '';");
    expect(out).toContain("const WASM_JS_URL = '';");
  });

  it('refuses a loader whose shape changed', () => {
    expect(() => stripBundledTranscoder('export class KTX2Loader {}')).toThrow(/found 0/);
  });

  it('keeps the public copy in step with the installed three', () => {
    for (const file of ['basis_transcoder.js', 'basis_transcoder.wasm']) {
      const shipped = readFileSync(resolve(import.meta.dirname, '../public/basis', file));
      const upstream = readFileSync(resolve(three, 'examples/jsm/libs/basis', file));
      expect(shipped.equals(upstream), `${file} differs from three's copy`).toBe(true);
    }
  });
});
