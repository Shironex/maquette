// Game export: lay the exported models out the way the game keeps them and
// check that its engine can read every one.
//
// Maquette runs this as the Game export step of a game project, with an
// args array (no shell):
//
//   node tools/game_export.mjs --index web/game-index.json --raw web/raw \
//     --out web/game --report web/game-export.json
//
// --out is emptied, then filled exactly as the game's model folder is
// (props/brazier.glb), which is what Ship writes into the game:
// - every file Export wrote to --raw (the models with work done on them,
//   with an .obj's .mtl and textures beside it);
// - for each model the index marks "asIs" (still the repo file it was
//   imported from, nothing changed), its "copy" files, byte for byte, so
//   Ship finds them unchanged.
//
// The checks, per .glb: no KTX2 (KHR_texture_basisu), no meshopt
// (EXT_meshopt_compression) and no Draco, textures only PNG or JPG, and for
// an exported model every texture within the index's "textureCap". An
// exported model that fails stops the step; an untouched import that does
// is only reported, since it is the game's own file. --report lists every
// file with its size and what the checks found.

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { parseArgs } from 'node:util';

const INDEX_VERSION = 1;
const FORBIDDEN = {
  KHR_texture_basisu: 'KTX2',
  EXT_meshopt_compression: 'meshopt',
  KHR_draco_mesh_compression: 'Draco',
};

const { values } = parseArgs({
  options: {
    index: { type: 'string', default: 'web/game-index.json' },
    raw: { type: 'string', default: 'web/raw' },
    out: { type: 'string', default: 'web/game' },
    report: { type: 'string', default: 'web/game-export.json' },
  },
});

const log = message => console.log(`[game_export] ${message}`);
const fail = message => {
  console.error(`[game_export] ${message}`);
  process.exit(1);
};

function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    return fail(`${path} could not be read: ${error.message}`);
  }
}

/** Every file under `dir`, relative to it with `/`. */
function filesUnder(dir) {
  if (!existsSync(dir)) return [];
  const out = [];
  const walk = (folder, prefix) => {
    for (const name of readdirSync(folder).sort()) {
      if (name.startsWith('.')) continue;
      const path = join(folder, name);
      const rel = prefix ? `${prefix}/${name}` : name;
      if (statSync(path).isDirectory()) walk(path, rel);
      else out.push(rel);
    }
  };
  walk(dir, '');
  return out;
}

function copy(from, to) {
  mkdirSync(dirname(to), { recursive: true });
  copyFileSync(from, to);
}

/** Width and height from a PNG or JPEG header. */
function imageSize(bytes) {
  if (bytes.length > 24 && bytes.readUInt32BE(0) === 0x89504e47) {
    return [bytes.readUInt32BE(16), bytes.readUInt32BE(20)];
  }
  if (bytes[0] === 0xff && bytes[1] === 0xd8) {
    let at = 2;
    while (at + 9 < bytes.length) {
      if (bytes[at] !== 0xff) return null;
      const marker = bytes[at + 1];
      const length = bytes.readUInt16BE(at + 2);
      if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker)) {
        return [bytes.readUInt16BE(at + 7), bytes.readUInt16BE(at + 5)];
      }
      at += 2 + length;
    }
  }
  return null;
}

/** What one .glb holds that the checks look at. */
function inspectGlb(path) {
  const data = readFileSync(path);
  if (data.toString('ascii', 0, 4) !== 'glTF') return { error: 'not a binary glTF file' };
  const length = data.readUInt32LE(12);
  const doc = JSON.parse(data.toString('utf8', 20, 20 + length));
  const binStart = 20 + length + 8;
  const textures = (doc.images ?? []).map(image => {
    const view = doc.bufferViews?.[image.bufferView];
    const bytes = view
      ? data.subarray(
          binStart + (view.byteOffset ?? 0),
          binStart + (view.byteOffset ?? 0) + view.byteLength
        )
      : null;
    const size = bytes ? imageSize(bytes) : null;
    return { name: image.name ?? '', mime: image.mimeType ?? null, size };
  });
  return { extensions: doc.extensionsUsed ?? [], textures };
}

/** The problems of one .glb for a game engine. */
function problems(info, cap) {
  if (info.error) return [info.error];
  const found = info.extensions.filter(e => e in FORBIDDEN).map(e => `uses ${FORBIDDEN[e]}`);
  for (const texture of info.textures) {
    if (texture.mime && !['image/png', 'image/jpeg'].includes(texture.mime)) {
      found.push(`${texture.name || 'a texture'} is ${texture.mime}, not PNG or JPG`);
    }
    const edge = texture.size ? Math.max(...texture.size) : 0;
    if (edge > cap) found.push(`${texture.name || 'a texture'} is ${edge} px, over ${cap} px`);
  }
  return found;
}

const index = readJson(values.index);
if ((index.version ?? 1) > INDEX_VERSION) fail(`${values.index} comes from a newer Maquette`);
const cap = Number(index.textureCap ?? 1024);
rmSync(values.out, { recursive: true, force: true });
mkdirSync(values.out, { recursive: true });

const exported = filesUnder(values.raw).filter(f => f !== 'export.json');
for (const file of exported) copy(join(values.raw, file), join(values.out, file));
const rows = [];
let failed = 0;
for (const model of index.models ?? []) {
  const files = [];
  if (model.asIs) {
    const base = dirname(model.source);
    for (const from of model.copy ?? []) {
      const to =
        from === model.source ? model.target : join(dirname(model.target), relative(base, from));
      copy(from, join(values.out, to));
      files.push(to.replaceAll('\\', '/'));
    }
  } else if (existsSync(join(values.raw, model.target))) {
    files.push(model.target);
  } else {
    log(`${model.name}: Export wrote no ${model.target}`);
    failed += 1;
    continue;
  }
  const target = join(values.out, model.target);
  const found = model.target.endsWith('.glb') ? problems(inspectGlb(target), cap) : [];
  if (found.length && !model.asIs) failed += 1;
  const how = model.asIs ? 'the game file as it is' : 'exported';
  const bytes = statSync(target).size;
  log(
    `${model.target}  ${how}, ${(bytes / 1e6).toFixed(2)} MB` +
      (found.length ? `  ${found.join('; ')}` : '')
  );
  rows.push({
    asset: model.asset,
    target: model.target,
    asIs: Boolean(model.asIs),
    files,
    bytes,
    problems: found,
  });
}

mkdirSync(dirname(values.report), { recursive: true });
writeFileSync(
  values.report,
  `${JSON.stringify({ version: 1, textureCap: cap, models: rows }, null, 2)}\n`
);
if (failed) fail(`${failed} models cannot ship to the game as they are; see the lines above`);
log(
  `${rows.length} models in ${values.out}: textures PNG or JPG, at most ${cap} px, ` +
    'no KTX2, no meshopt'
);
