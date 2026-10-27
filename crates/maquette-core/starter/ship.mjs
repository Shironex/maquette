// Ship: copy the compressed models and the posters into the site, and write
// the site's scene data when the site asks for it.
//
// Maquette runs this as the Ship step, with an args array (no shell):
//
//   node tools/ship.mjs --models web/dist --posters web/posters \
//     --site {siteRepo} --index {shipIndex} \
//     --layout {project}/layout.json --materials {project}/materials.json
//
// {siteRepo} is the site from the project settings. Without one the step
// says so and passes, so a new project runs before it has a site.
//
// Where things go comes from the site's maquette.site.json:
//
//   { "version": 1, "models": "public/scene/models", "posters": null,
//     "data": "src/scene/shipped.json" }
//
// Every folder is relative to the site and must stay inside it; "posters":
// null copies none. A site without the file gets the models in
// --models-to (public/models) and the posters in --posters-to
// (public/posters), and no data.
//
// The data file lists each asset's shipped model ({shipIndex}, written by
// Maquette before the run), and carries layout.json and materials.json as
// they are, so the site places the models and frames its stops from them.
// The models are exported in place, so each is marked "inPlace". It is only
// written from a layout with placed models and camera stops: shipping an
// empty layout would wipe the site's scene, so the step stops first and
// copies nothing. With a data file only the models it lists are copied, so
// a leftover in --models never reaches the site. Files are copied, never
// moved.
//
// --record (web/ship-record.json) remembers which model files this project
// shipped into each site folder. A file shipped last time that this Ship
// no longer ships is removed from the site, and logged; every other file in
// the site is left alone, whoever put it there.
//
// Before a file is copied over one the site already has, it is checked
// (Maquette's ship plan shows the same answer):
// - an asset whose "choice" in the index is "never" is held back, and one
//   set to "always" is copied whatever the site has;
// - a site file the record does not list was not made by Maquette, so it
//   is held back;
// - with "sizeGuard" in the index (a hand-built site), a site file that is
//   smaller than the new one is held back too.
// A held file is neither copied nor removed, and is logged. With
// "origin": "own" in the index the models are exported at their own
// origin, so the scene data marks them "inPlace": false.

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const CONFIG_FILE = 'maquette.site.json';
const DATA_VERSION = 1;
const RECORD_VERSION = 1;

const { values } = parseArgs({
  options: {
    models: { type: 'string', default: 'web/dist' },
    posters: { type: 'string', default: 'web/posters' },
    site: { type: 'string', default: '' },
    index: { type: 'string', default: '' },
    layout: { type: 'string', default: 'layout.json' },
    materials: { type: 'string', default: 'materials.json' },
    record: { type: 'string', default: 'web/ship-record.json' },
    'models-to': { type: 'string', default: 'public/models' },
    'posters-to': { type: 'string', default: 'public/posters' },
  },
});

const log = message => console.log(`[ship] ${message}`);
const fail = message => {
  console.error(`[ship] ${message}`);
  process.exit(1);
};

const site = values.site.trim();
if (!site || site.includes('{')) {
  log('no site repo is set for this project, nothing shipped. Pick one in Settings.');
  process.exit(0);
}
if (!existsSync(site) || !statSync(site).isDirectory()) {
  fail(`the site repo ${site} does not exist`);
}

function readJson(path, fallback) {
  if (!existsSync(path)) return fallback;
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    return fail(`${path} is not valid JSON: ${error.message}`);
  }
}

/** A path inside the site, refusing paths that leave it. */
function inside(folder) {
  const target = resolve(site, folder);
  const rel = relative(resolve(site), target);
  if (!rel || rel.startsWith('..') || isAbsolute(rel)) fail(`${folder} is outside the site repo`);
  return target;
}

/** The site's folders: its maquette.site.json, else the --*-to folders. */
function siteConfig() {
  const config = readJson(join(site, CONFIG_FILE), null);
  if (config === null) {
    return { models: values['models-to'], posters: values['posters-to'], data: null };
  }
  if (typeof config !== 'object' || (config.version ?? 1) > 1) {
    fail(`${CONFIG_FILE} is from a newer Maquette or is not a site config`);
  }
  return {
    models: config.models ?? 'public/models',
    posters: config.posters ?? null,
    data: config.data ?? null,
  };
}

/** The files in `from` whose names end with one of `exts`. */
function filesIn(from, exts) {
  if (!existsSync(from)) return [];
  return readdirSync(from).filter(name => exts.some(ext => name.endsWith(ext)));
}

function copyAll(from, to, files) {
  if (files.length === 0) return 0;
  mkdirSync(to, { recursive: true });
  for (const name of files) {
    copyFileSync(join(from, name), join(to, name));
    log(`${join(from, name)} to ${join(to, name)}`);
  }
  return files.length;
}

/** The ship index Maquette wrote before the run, or null without one. */
function readIndex(required) {
  if (!values.index || values.index.includes('{')) {
    if (!required) return null;
    fail('the Ship step has no --index {shipIndex}; update its args from the starter');
  }
  const index = readJson(values.index, null);
  if (!index || typeof index.models !== 'object') {
    if (!required && index === null) return null;
    fail(`${values.index} is not a ship index`);
  }
  return index;
}

/** The scene data the site reads, or a reason it cannot be written. */
function sceneData(config, shipped, index, present) {
  const layout = readJson(values.layout, { version: 1, models: {}, stops: [], desk: null });
  const materials = readJson(values.materials, { version: 1, models: {} });
  const placed = Object.keys(layout.models ?? {}).length;
  const stops = (layout.stops ?? []).length;
  if (placed === 0 || stops === 0) {
    fail(
      `the layout has ${placed} placed models and ${stops} camera stops. Place the models and ` +
        'add the stops in the Inspector, then ship again; nothing was copied.'
    );
  }
  const models = {};
  for (const [asset, entry] of Object.entries(index.models)) {
    if (!shipped.includes(entry.file) && !present(entry.file)) {
      log(`${asset}: ${entry.file} is not in ${values.models}, left out`);
      continue;
    }
    models[asset] = { name: entry.name, file: entry.file, inPlace: index.origin !== 'own' };
  }
  const posterList = config.posters
    ? (readJson(join(values.posters, 'stops.json'), { stops: [] }).stops ?? []).map(stop => ({
        file: stop.file,
        ...(stop.label ? { label: stop.label } : {}),
      }))
    : null;
  return {
    version: DATA_VERSION,
    project: index.project,
    models,
    layout,
    materials,
    posters: config.posters ? { folder: config.posters, stops: posterList } : null,
  };
}

/** Writes `data` next to its target first, so the site never reads half a file. */
function writeData(path, data) {
  mkdirSync(dirname(path), { recursive: true });
  const temp = `${path}.ship-tmp`;
  writeFileSync(temp, `${JSON.stringify(data, null, 2)}\n`);
  renameSync(temp, path);
  log(`wrote ${Object.keys(data.models).length} models and the layout to ${path}`);
}

/** The model files this project shipped into each site folder, by absolute path. */
function readRecord() {
  const record = readJson(values.record, null);
  if (!record || typeof record.targets !== 'object' || (record.version ?? 1) > RECORD_VERSION) {
    return { version: RECORD_VERSION, targets: {} };
  }
  return record;
}

/** True when two files hold the same bytes. */
function sameBytes(a, b) {
  return statSync(a).size === statSync(b).size && readFileSync(a).equals(readFileSync(b));
}

/** Why `name` must not be copied into `to`, or null when it may. */
function holdReason(index, owned, to, name) {
  const entry = Object.values(index?.models ?? {}).find(m => m.file === name);
  const choice = entry?.choice ?? 'auto';
  if (choice === 'never') return 'set to never ship';
  const target = join(to, name);
  if (choice === 'always' || !existsSync(target)) return null;
  if (sameBytes(join(values.models, name), target)) return null;
  if (!owned.includes(name)) return 'the site file was not made by Maquette';
  const grows = statSync(join(values.models, name)).size > statSync(target).size;
  if (index?.sizeGuard && grows) return 'the site file is smaller';
  return null;
}

/** Removes the files this project shipped to `to` last time and no longer ships. */
function dropStale(record, to, files, held) {
  const before = Array.isArray(record.targets[to]) ? record.targets[to] : [];
  let removed = 0;
  for (const name of before) {
    // Only plain file names it wrote itself, never a path.
    if (files.includes(name) || held.includes(name)) continue;
    if (name !== basename(name) || !name.endsWith('.glb')) continue;
    const path = join(to, name);
    if (!existsSync(path)) continue;
    rmSync(path);
    removed += 1;
    log(`removed ${path}: shipped last time, no longer in the scene`);
  }
  return removed;
}

function writeRecord(record, to, files) {
  record.targets[to] = [...files].sort();
  mkdirSync(dirname(values.record), { recursive: true });
  writeFileSync(values.record, `${JSON.stringify(record, null, 2)}\n`);
}

const config = siteConfig();
const built = filesIn(values.models, ['.glb']);
if (built.length === 0) fail(`there are no .glb files in ${values.models} to ship`);
// Everything is checked before anything is copied.
const modelsTo = inside(config.models);
const postersTo = config.posters ? inside(config.posters) : null;
const dataTo = config.data ? inside(config.data) : null;
const index = readIndex(Boolean(dataTo));
const record = readRecord();
const owned = Array.isArray(record.targets[modelsTo]) ? record.targets[modelsTo] : [];
const held = built.filter(name => {
  const why = holdReason(index, owned, modelsTo, name);
  if (why) log(`${name} held back: ${why}`);
  return why !== null;
});
const present = name => existsSync(join(modelsTo, name));
const copyable = built.filter(name => !held.includes(name));
const data = dataTo ? sceneData(config, copyable, index, present) : null;
// With scene data the site gets exactly the models the data lists.
const wanted = data ? [...new Set(Object.values(data.models).map(m => m.file))] : built;
for (const name of built.filter(name => !wanted.includes(name))) {
  log(`${join(values.models, name)} is in no shipped asset, not copied`);
}
const shipped = wanted.filter(name => built.includes(name) && !held.includes(name));
const kept = held.filter(name => owned.includes(name));

const stale = dropStale(record, modelsTo, shipped, held);
const models = copyAll(values.models, modelsTo, shipped);
writeRecord(record, modelsTo, [...shipped, ...kept]);
const posters = postersTo
  ? copyAll(values.posters, postersTo, filesIn(values.posters, ['.png', '.jpg', '.json']))
  : 0;
if (dataTo && data) writeData(dataTo, data);
log(
  `shipped ${models} models and ${posters} poster files to ${site}` +
    (stale ? `, removed ${stale} it shipped before` : '') +
    (held.length ? `, held back ${held.length}` : '')
);
