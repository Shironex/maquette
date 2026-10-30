/**
 * `?library=scale`: the library a long round leaves behind. The demo
 * entries plus generated village, forest and market models (214 in all),
 * three collections, models with no tag, two with no picture, look-alike
 * and same name pairs and one merge that can still be undone.
 */
import type {
  LibraryCollection,
  LibraryEntry,
  LibraryMerge,
  LibrarySource,
  LibraryVersion,
} from '../client';
import { clock } from '@/lib/clock';
import { HOLLOW_VALE, SAME_FILE_IDS } from './hollow-vale-fixtures';
import { libraryEntries } from './library-fixtures';

const DAY = 86_400_000;
export const SCALE_TOTAL = 214;

const VILLAGE = [
  'Cast iron stove',
  'Kitchen hutch',
  'Farm table',
  'Writing desk',
  'Bookshelf',
  'Rocking chair',
  'Long clock',
  'Brass globe',
  'Street lamp',
  'Stone well',
  'Copper pots',
  'Flower basket',
  'Wheelbarrow',
  'Garden tools',
  'Watering can',
  'Stone planter',
  'Bee hive',
  'Water trough',
  'Candelabrum',
  'Paper lanterns',
  'Stone lantern',
  'Hay bales',
  'Market stall',
  'Flower cart',
  'Street lamp 2',
  'Stone well 2',
  'Hive box',
  'Butter churn',
  'Spinning wheel',
  'Wash tub',
  'Bread oven',
  'Milk cans',
  'Apple crate',
  'Herb rack',
  'Tea kettle',
  'Quilt chest',
  'Lantern hook',
  'Door wreath',
  'Coal scuttle',
  'Pie safe',
];
const FOREST = [
  'Mossy log',
  'Fern clump',
  'Toadstool ring',
  'Birch stump',
  'Fallen pine',
  'Berry bush',
  'Fox den',
  'Hollow tree',
  'Owl post',
  'Woodpile',
  'Forest shrine',
  'Rope bridge',
  'Trail sign',
  'Camp stool',
  'Bedroll',
  'Cook pot',
  'Hunting blind',
  'Stone cairn',
  'Creek stones',
  'Lily pads',
  'Reed bundle',
  'Moss boulder',
  'Root arch',
  'Acorn pile',
  'Spider web',
  'Mushroom cap',
  'Pine cone heap',
  'Badger sett',
  'Forest gate',
  'Thorn hedge',
];
const MARKET = [
  'Fish stand',
  'Cheese wheel',
  'Spice sacks',
  'Cloth bolts',
  'Fruit stall',
  'Scale and weights',
  'Pottery shelf',
  'Rug pile',
  'Bread basket',
  'Wine barrels',
  'Candle stall',
  'Flower bucket',
  'Awning post',
  'Coin box',
  'Hanging meat',
  'Egg crate',
  'Honey jars',
  'Sausage rack',
];
const WOODS = [
  'Oak',
  'Pine',
  'Birch',
  'Iron',
  'Stone',
  'Clay',
  'Brass',
  'Willow',
  'Frosted',
  'Painted',
];
const THINGS = [
  'barrel',
  'crate',
  'bench',
  'stool',
  'fence',
  'sign',
  'basket',
  'jar',
  'bucket',
  'cart',
  'shelf',
  'chest',
  'post',
  'gate',
  'trellis',
];

/** The demo renders, in turn, as stand in pictures. */
const RENDERS = [
  'crystal-lantern.webp',
  'guardian-statue.webp',
  'observatory-arch.webp',
  'rune-obelisk.webp',
  'veil-gate.webp',
  'crypt-brazier.webp',
  'frozen-angel.webp',
  'ice-crystals.webp',
  'veil-platform.webp',
  'elsheria.webp',
];
/** Pairs that share a picture, so their cards look alike too. */
const SAME_PICTURE: Record<string, string> = {
  'street-lamp-2': 'street-lamp',
  'hive-box': 'bee-hive',
};
/** Models whose file moved, so they have no picture. */
export const NO_PICTURE = new Set(['spider-web', 'oak-trellis']);

/** How alike two entries look, by outline, colour and box ratio. */
export const SCALE_LOOKS: readonly {
  a: string;
  b: string;
  shape: number;
  colour: number;
  ratio: number;
}[] = [
  { a: 'street-lamp', b: 'street-lamp-2', shape: 0.97, colour: 0.94, ratio: 0.98 },
  { a: 'bee-hive', b: 'hive-box', shape: 0.9, colour: 0.9, ratio: 0.95 },
  { a: 'stone-well', b: 'stone-well-2', shape: 0.38, colour: 0.42, ratio: 0.46 },
];

const slug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
const key = (id: string) => id.replace(/-/g, '_');

/** A deterministic number from 0 to 1 for the n-th model. */
const noise = (n: number, salt: number) => ((n * 9301 + salt * 49297) % 233280) / 233280;

interface Made {
  name: string;
  tags: string[];
  collection: 'village-props' | 'forest-kit' | 'market-set' | null;
}

function names(): Made[] {
  const made: Made[] = [
    ...VILLAGE.map(name => ({
      name,
      tags: villageTags(name),
      collection: 'village-props' as const,
    })),
    ...FOREST.map(name => ({ name, tags: ['forest', 'plant'], collection: 'forest-kit' as const })),
    ...MARKET.map(name => ({ name, tags: ['market', 'food'], collection: 'market-set' as const })),
  ];
  for (const wood of WOODS) {
    for (const thing of THINGS)
      made.push({ name: `${wood} ${thing}`, tags: ['prop'], collection: null });
  }
  return made;
}

function villageTags(name: string): string[] {
  if (/lamp|lantern|candel/i.test(name)) return ['light'];
  if (/stove|hutch|table|desk|shelf|chair|clock|chest|safe/i.test(name)) return ['furniture'];
  if (/pots|kettle|churn|oven|milk|crate|rack/i.test(name)) return ['kitchen'];
  return ['prop'];
}

function source(n: number): LibrarySource {
  if (n % 13 === 0) return 'built';
  return n % 4 === 0 ? 'meshy' : 'tripo';
}

function version(id: string, n: number, i: number, from: string): LibraryVersion {
  const made = source(n);
  const triangles = Math.round(1_500 + noise(n, i + 1) * 38_000);
  return {
    version: i + 1,
    file: `models/${key(id)}_v${i + 1}.glb`,
    createdAt: new Date(clock.nowMs() - (60 - (n % 58) - i) * DAY).toISOString(),
    stats: {
      triangles,
      bytes: triangles * 34,
      textures: 2,
      maxTexture: 1024,
      joints: 0,
      animations: 0,
    },
    source: made,
    credits: made === 'built' ? null : made === 'meshy' ? 20 : 30,
    // A few came from Hollow Vale before, as the very file it still has.
    hash: SAME_FILE_IDS.has(id) ? `${HOLLOW_VALE}/models/${key(id)}_v1.glb` : `${id}-${i + 1}`,
    from: { project: from, asset: id, version: i + 1 },
    thumbnail: NO_PICTURE.has(id) ? null : `thumbs/${key(id)}_v${i + 1}.webp`,
    prep: null,
    materials: null,
  };
}

/** The generated entries, in the order made, cut to fill the library to `SCALE_TOTAL`. */
function generated(room: number): (LibraryEntry & { collection: Made['collection'] })[] {
  return names()
    .slice(0, room)
    .map((m, n) => {
      const id = slug(m.name);
      const count = n % 7 === 0 ? 2 : 1;
      const versions = Array.from({ length: count }, (_, i) => version(id, n, i, 'frostveil'));
      // About one model in five was added without a tag.
      const tags = n % 5 === 2 ? [] : m.tags;
      return {
        version: 1,
        id,
        name: m.name,
        kind: 'prop' as const,
        tags,
        refs: [],
        versions,
        createdAt: versions[0]!.createdAt,
        updatedAt: versions.at(-1)!.createdAt,
        collection: m.collection,
      };
    });
}

export interface ScaleLibrary {
  entries: LibraryEntry[];
  collections: LibraryCollection[];
  /** The merged entry, kept to undo, and the merge as the look-alikes list shows it. */
  merged: { entry: LibraryEntry; merge: LibraryMerge }[];
}

export function scaleLibrary(): ScaleLibrary {
  const demo = libraryEntries();
  const made = generated(SCALE_TOTAL - demo.length);
  const now = new Date(clock.nowMs()).toISOString();
  const collection = (id: string, name: string): LibraryCollection => ({
    id,
    name,
    entries: made.filter(e => e.collection === id).map(e => e.id),
    createdAt: now,
  });
  const lanterns = made.find(e => e.id === 'paper-lanterns')!;
  const mergedEntry: LibraryEntry = {
    ...lanterns,
    id: 'paper-lanterns-2',
    name: 'Paper lanterns 2',
  };
  return {
    entries: [...demo, ...made.map(({ collection: _, ...e }) => e)],
    collections: [
      collection('village-props', 'Village props'),
      collection('forest-kit', 'Forest kit'),
      collection('market-set', 'Market set'),
    ],
    merged: [
      {
        entry: mergedEntry,
        merge: {
          merged: 'paper-lanterns-2',
          mergedName: 'Paper lanterns 2',
          into: 'paper-lanterns',
          intoName: 'Paper lanterns',
          versions: [lanterns.versions.length],
          at: new Date(clock.nowMs() - 40 * 60_000).toISOString(),
        },
      },
    ],
  };
}

/** Each generated entry's picture under the library folder, to a demo render. */
export function scaleLibraryFiles(): Record<string, string> {
  const out: Record<string, string> = {};
  const made = generated(SCALE_TOTAL);
  const render = new Map(made.map((e, n) => [e.id, RENDERS[n % RENDERS.length]!]));
  for (const e of made) {
    const picture = render.get(SAME_PICTURE[e.id] ?? e.id)!;
    for (const v of e.versions) if (v.thumbnail) out[`${e.id}/${v.thumbnail}`] = picture;
  }
  return out;
}

/** The bundled models the quick view shows for the generated entries, in turn. */
export function scaleQuickViewFiles(
  root: string,
  models: readonly string[]
): Record<string, string> {
  const out: Record<string, string> = {};
  generated(SCALE_TOTAL).forEach((e, n) => {
    for (const v of e.versions)
      out[`${root}/library/${e.id}/${v.file}`] = models[n % models.length]!;
  });
  return out;
}
