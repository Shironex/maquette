/**
 * `?library=scale` brings Hollow Vale: a project with 121 accepted models,
 * ready to add to the library in one go. Six of them are already there as
 * the same file, three meet a library model by name (one looks the same,
 * one does not, one nearly), two files are cut short, and the rest are new.
 */
import type { Asset, AssetKind, ProjectManifest } from '../client';
import { clock } from '@/lib/clock';

const DAY = 86_400_000;
export const HOLLOW_VALE = 'hollow-vale';

const CLASHES = ['Street lamp', 'Stone well', 'Paper lanterns'];
/** How alike each clash's picture is to the library model it meets. */
export const HOLLOW_LOOKS: Record<string, number> = {
  'street-lamp': 0.96,
  'stone-well': 0.41,
  'paper-lanterns': 0.93,
};
/** Models the library already has as the very same file (see library-scale-fixtures.ts). */
const SAME_FILE = [
  'Wheelbarrow',
  'Garden tools',
  'Watering can',
  'Hay bales',
  'Water trough',
  'Flower cart',
];
export const SAME_FILE_IDS = new Set(SAME_FILE.map(n => n.toLowerCase().replace(/ /g, '-')));
/** Files that end before their header says. */
export const CUT_SHORT = new Set(['barrel-stack', 'fence-corner']);
const BROKEN = ['Barrel stack', 'Fence corner'];

const PLACES = [
  'Mill',
  'Inn',
  'Forge',
  'Chapel',
  'Bakery',
  'Farm',
  'Dock',
  'Smithy',
  'Tavern',
  'Garden',
  'Square',
];
const PARTS = [
  'door',
  'window',
  'roof',
  'bench',
  'lamp',
  'sign',
  'table',
  'barrel',
  'crate',
  'fence',
];
const FOLK = ['Miller', 'Innkeeper', 'Smith', 'Baker', 'Farmer', 'Fisher'];

const slug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
const key = (id: string) => id.replace(/-/g, '_');

function asset(name: string, kind: AssetKind, n: number): Asset {
  const id = slug(name);
  const at = new Date(clock.nowMs() - (n % 9) * DAY).toISOString();
  const triangles = 2_000 + ((n * 7919) % 30_000);
  return {
    id,
    name,
    kind,
    stage: 'blender',
    status: 'done',
    versions: [
      {
        version: 1,
        file: `models/${key(id)}_v1.glb`,
        createdAt: at,
        stats: {
          triangles,
          bytes: triangles * 34,
          textures: 2,
          maxTexture: 1024,
          joints: kind === 'character' ? 52 : 0,
          animations: 0,
        },
      },
    ],
    thumbnail: null,
    createdAt: at,
    updatedAt: at,
  };
}

function names(): [string, AssetKind][] {
  const out: [string, AssetKind][] = [
    ...CLASHES.map((n): [string, AssetKind] => [n, 'prop']),
    ...SAME_FILE.map((n): [string, AssetKind] => [n, 'prop']),
    ...BROKEN.map((n): [string, AssetKind] => [n, 'prop']),
    ...FOLK.map((n): [string, AssetKind] => [n, 'character']),
    ['Village green', 'scene'],
  ];
  for (const place of PLACES) for (const part of PARTS) out.push([`${place} ${part}`, 'prop']);
  return out.slice(0, 121);
}

export function hollowVale(): ProjectManifest {
  return {
    version: 1,
    id: HOLLOW_VALE,
    name: 'Hollow Vale',
    template: 'floating-diorama',
    target: 'website',
    siteRepo: null,
    sourceFolder: null,
    createdAt: new Date(clock.nowMs() - 12 * DAY).toISOString(),
    assets: names().map(([name, kind], n) => asset(name, kind, n)),
  };
}
