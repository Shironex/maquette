/**
 * One copy of the Basis transcoder. three's KTX2Loader points at its own
 * `libs/basis` files through `new URL(..., import.meta.url)`, which Vite
 * copies into every build as hashed assets, while the preview loads the
 * copy in `public/basis` (see `TRANSCODER` in `src/features/viewer/studio.ts`).
 * This plugin blanks those two default URLs, so the build ships only the
 * public copy, in dev and in production alike.
 */
import type { Plugin } from 'vite';

const LOADER = /three\/examples\/jsm\/loaders\/KTX2Loader\.js$/;
const DEFAULT_URL =
  /new URL\(\s*'\.\.\/libs\/basis\/basis_transcoder\.(?:wasm|js)',\s*import\.meta\.url\s*\)\.toString\(\)/g;

/** The loader's source with its bundled transcoder URLs blanked. */
export function stripBundledTranscoder(code: string): string {
  const found = code.match(DEFAULT_URL)?.length ?? 0;
  if (found !== 2) {
    // A three upgrade changed the loader: fail loudly instead of shipping two copies.
    throw new Error(
      `KTX2Loader: expected the two default transcoder URLs, found ${found}. Update vite/basis-transcoder.ts.`
    );
  }
  return code.replace(DEFAULT_URL, "''");
}

export function singleBasisTranscoder(): Plugin {
  return {
    name: 'maquette:single-basis-transcoder',
    enforce: 'pre',
    transform(code, id) {
      if (!LOADER.test(id.split('?')[0] ?? id)) return null;
      return { code: stripBundledTranscoder(code), map: null };
    },
  };
}
