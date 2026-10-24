// Ship: copy the compressed models and the posters into the site repo.
//
// Maquette runs this as the Ship step, with an args array (no shell):
//
//   node tools/ship.mjs --models web/dist --posters web/posters \
//     --site {siteRepo} --models-to public/models --posters-to public/posters
//
// {siteRepo} is the site repo from the project settings. Without one the
// step says so and passes, so a new project runs before it has a site. The
// --models-to and --posters-to folders are relative to the site repo and
// must stay inside it. Files are copied, never moved, and files the site
// already has with other names are left alone.

import { copyFileSync, existsSync, mkdirSync, readdirSync, statSync } from 'node:fs';
import { isAbsolute, join, relative, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const { values } = parseArgs({
  options: {
    models: { type: 'string', default: 'web/dist' },
    posters: { type: 'string', default: 'web/posters' },
    site: { type: 'string', default: '' },
    'models-to': { type: 'string', default: 'public/models' },
    'posters-to': { type: 'string', default: 'public/posters' },
  },
});

const log = message => console.log(`[ship] ${message}`);

const site = values.site.trim();
if (!site || site.includes('{')) {
  log('no site repo is set for this project, nothing shipped. Pick one in Settings.');
  process.exit(0);
}
if (!existsSync(site) || !statSync(site).isDirectory()) {
  console.error(`[ship] the site repo ${site} does not exist`);
  process.exit(1);
}

/** A folder inside the site repo, refusing paths that leave it. */
function inside(folder) {
  const target = resolve(site, folder);
  const rel = relative(resolve(site), target);
  if (rel.startsWith('..') || isAbsolute(rel)) {
    console.error(`[ship] ${folder} is outside the site repo`);
    process.exit(1);
  }
  return target;
}

/** Copies the files in `from` whose names end with one of `exts` into `to`. */
function copyAll(from, to, exts) {
  if (!existsSync(from)) return 0;
  const files = readdirSync(from).filter(name => exts.some(ext => name.endsWith(ext)));
  if (files.length === 0) return 0;
  mkdirSync(to, { recursive: true });
  for (const name of files) {
    copyFileSync(join(from, name), join(to, name));
    log(`${join(from, name)} to ${join(to, name)}`);
  }
  return files.length;
}

const models = copyAll(values.models, inside(values['models-to']), ['.glb']);
const posters = copyAll(values.posters, inside(values['posters-to']), ['.png', '.jpg', '.json']);
if (models === 0) {
  console.error(`[ship] there are no .glb files in ${values.models} to ship`);
  process.exit(1);
}
log(`shipped ${models} models and ${posters} poster files to ${site}`);
