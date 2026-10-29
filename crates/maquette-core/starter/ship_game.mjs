// Ship to a game: write the Game export into the game's model folder,
// replacing only the files Maquette owns there.
//
// Maquette runs this as the Ship step of a game project, with an args
// array (no shell), after you approve it in the app:
//
//   node tools/ship_game.mjs --from web/game --game {siteRepo} \
//     --models assets/models --record web/ship-record.json
//
// {siteRepo} is the game repo from the project settings. Without one the
// step says so and passes.
//
// Each file in --from (props/brazier.glb) goes to the same path under the
// model folder. A file already there with the same bytes is left as it is.
// A different file is only replaced when the ship record lists it: a file
// Maquette wrote there before, or one imported from there. Any other file
// is hand made: it is left alone, also when an export would land on it,
// and the log says so. Ship never commits; review the change in git and
// commit it yourself.
//
// Ship removes one kind of file only: the old file of an asset moved to
// another folder in Maquette. The move lists it under "remove" in the
// record; Ship deletes it when the record still owns it and no export
// lands on it, then drops it from the record. Nothing else is removed.
//
// The record (--record) lists, under the model folder, every file Maquette
// owns there; the files this Ship writes join it:
//
//   { "version": 1, "targets": { "assets/models": ["props/brazier.glb"] } }

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const RECORD_VERSION = 1;

const { values } = parseArgs({
  options: {
    from: { type: 'string', default: 'web/game' },
    game: { type: 'string', default: '' },
    models: { type: 'string', default: 'assets/models' },
    record: { type: 'string', default: 'web/ship-record.json' },
  },
});

const log = message => console.log(`[ship_game] ${message}`);
const fail = message => {
  console.error(`[ship_game] ${message}`);
  process.exit(1);
};

const game = values.game.trim();
if (!game || game.includes('{')) {
  log('no game repo is set for this project, nothing shipped. Pick one in Settings.');
  process.exit(0);
}
if (!existsSync(game) || !statSync(game).isDirectory())
  fail(`the game repo ${game} does not exist`);
const target = resolve(game, values.models);
const inside = relative(resolve(game), target);
if (!inside || inside.startsWith('..') || isAbsolute(inside)) {
  fail(`${values.models} is outside the game repo`);
}
if (!existsSync(values.from)) fail(`there is no ${values.from}; run Game export first`);

/** Every file under `dir`, relative to it with `/`. */
function filesUnder(dir) {
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

function readRecord() {
  if (!existsSync(values.record)) return { version: RECORD_VERSION, targets: {} };
  try {
    const record = JSON.parse(readFileSync(values.record, 'utf8'));
    if (typeof record.targets !== 'object' || (record.version ?? 1) > RECORD_VERSION) {
      fail(`${values.record} comes from a newer Maquette or is not a ship record`);
    }
    return record;
  } catch (error) {
    return fail(`${values.record} is not valid JSON: ${error.message}`);
  }
}

const record = readRecord();
const key = values.models;
const owned = new Set(Array.isArray(record.targets[key]) ? record.targets[key] : []);
const counts = { new: 0, replaced: 0, unchanged: 0, handMade: 0, removed: 0 };
const produced = filesUnder(values.from);
for (const file of produced) {
  const from = join(values.from, file);
  const to = join(target, file);
  if (existsSync(to)) {
    if (readFileSync(from).equals(readFileSync(to))) {
      counts.unchanged += 1;
      continue;
    }
    if (!owned.has(file)) {
      counts.handMade += 1;
      log(`${file} is hand made (not in the ship record), left alone`);
      continue;
    }
    counts.replaced += 1;
    log(`replaced ${file}`);
  } else {
    counts.new += 1;
    log(`new ${file}`);
  }
  mkdirSync(dirname(to), { recursive: true });
  // Next to the target first, so the game never reads half a file.
  const temp = `${to}.ship-tmp`;
  copyFileSync(from, temp);
  renameSync(temp, to);
  owned.add(file);
}

// The old files of moved assets, when Maquette still owns them.
const moved = Array.isArray(record.remove?.[key]) ? record.remove[key] : [];
for (const file of moved) {
  if (typeof file !== 'string' || !owned.has(file) || produced.includes(file)) continue;
  const at = resolve(target, file);
  const within = relative(target, at);
  if (!within || within.startsWith('..') || isAbsolute(within)) continue;
  if (existsSync(at) && statSync(at).isFile()) {
    unlinkSync(at);
    counts.removed += 1;
    log(`removed ${file}, its asset moved to another folder`);
  }
  owned.delete(file);
}
if (record.remove) {
  delete record.remove[key];
  if (Object.keys(record.remove).length === 0) delete record.remove;
}

record.version = RECORD_VERSION;
record.targets[key] = [...owned].sort();
mkdirSync(dirname(values.record), { recursive: true });
writeFileSync(values.record, `${JSON.stringify(record, null, 2)}\n`);
log(
  `shipped to ${join(game, values.models)}: ${counts.new} new, ${counts.replaced} replaced, ` +
    `${counts.unchanged} unchanged, ${counts.handMade} hand made left alone, ` +
    `${counts.removed} moved away. Nothing was ` +
    'committed: review the change in git and commit it yourself.'
);
