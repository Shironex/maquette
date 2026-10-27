/**
 * Serves three's Basis transcoder at basis/ under the site's base in dev and
 * emits it there in the build, straight from the installed three package, so
 * the site keeps no copy of its own and the transcoder always matches the
 * loader. The loader's bundled default URLs are blanked by the web app's
 * plugin, so the build ships one copy only.
 */
import { createReadStream, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import type { Plugin } from 'vite';
import { singleBasisTranscoder } from '../../web/vite/basis-transcoder.ts';

const FILES = {
  'basis_transcoder.js': 'text/javascript',
  'basis_transcoder.wasm': 'application/wasm',
} as const;

type TranscoderFile = keyof typeof FILES;

function basisDir(): string {
  const require = createRequire(import.meta.url);
  // three does not export its package.json; its main entry sits in build/.
  return resolve(dirname(require.resolve('three')), '../examples/jsm/libs/basis');
}

function isTranscoderFile(name: string): name is TranscoderFile {
  return Object.hasOwn(FILES, name);
}

export function basisTranscoder(): Plugin[] {
  const dir = basisDir();
  const serve: Plugin = {
    name: 'maquette-site:basis-transcoder',
    configureServer(server) {
      server.middlewares.use(
        `${server.config.base.replace(/\/+$/, '')}/basis/`,
        (req, res, next) => {
          const name = (req.url ?? '').split('?')[0]?.replace(/^\//, '') ?? '';
          if (!isTranscoderFile(name)) return next();
          res.setHeader('Content-Type', FILES[name]);
          createReadStream(resolve(dir, name)).pipe(res);
        }
      );
    },
    generateBundle(options) {
      // Astro also runs a server build; only the client build needs the files.
      if (options.format !== 'es' || this.environment?.name === 'ssr') return;
      for (const name of Object.keys(FILES)) {
        this.emitFile({
          type: 'asset',
          fileName: `basis/${name}`,
          source: readFileSync(resolve(dir, name)),
        });
      }
    },
  };
  return [singleBasisTranscoder(), serve];
}
