/**
 * Compare before and after a split: which two sides to show for an asset
 * and the table of their numbers, with a dot on the better one.
 */
import type { Asset, AssetParts, ModelParts, PartsSide } from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';

/**
 * The newest model as it came against after its split, when its fix-ups
 * split it; else the version before against the newest. Null with one
 * version and no split.
 */
export function sidesFor(asset: Asset, found: AssetParts): [PartsSide, PartsSide] | null {
  const newest = asset.versions.at(-1);
  if (!newest) return null;
  if (found.split) {
    return [
      { version: newest.version, fixed: false },
      { version: newest.version, fixed: true },
    ];
  }
  const before = asset.versions.at(-2);
  return before ? [{ version: before.version, fixed: false }, { version: newest.version }] : null;
}

/** How a side reads on its label: "v2 · one mesh", "v2 · split in Blender, 5 parts". */
export function sideLabel(side: PartsSide, numbers: ModelParts | null): string {
  const parts = numbers?.parts.length ?? 0;
  const what = parts ? `${parts} parts` : 'one mesh';
  return side.fixed ? `v${side.version} · split in Blender, ${what}` : `v${side.version} · ${what}`;
}

export interface CompareRow {
  label: string;
  left: string;
  right: string;
  /** The side with the better number, if one is better. */
  better: 'left' | 'right' | null;
}

const pick = (left: number, right: number, higher = false): CompareRow['better'] => {
  if (left === right) return null;
  return (higher ? left > right : left < right) ? 'left' : 'right';
};

const checks = (n: ModelParts) =>
  [
    `${n.checks.pass} pass`,
    n.checks.warning ? `${n.checks.warning} warning${n.checks.warning > 1 ? 's' : ''}` : '',
    n.checks.fail ? `${n.checks.fail} failing` : '',
  ]
    .filter(Boolean)
    .join(', ');

const texture = (n: ModelParts) => (n.textures ? `${n.textures} × ${n.maxTexture} px` : 'none');

/** The table under the two sides. */
export function compareRows(left: ModelParts, right: ModelParts): CompareRow[] {
  const trouble = (n: ModelParts) => n.checks.fail * 100 + n.checks.warning;
  return [
    {
      label: 'Parts Motion can move',
      left: left.parts.join(', ') || 'none',
      right: right.parts.join(', ') || 'none',
      better: pick(left.parts.length, right.parts.length, true),
    },
    {
      label: 'Draw calls',
      left: String(left.drawCalls),
      right: String(right.drawCalls),
      better: null,
    },
    {
      label: 'Triangles',
      left: formatK(left.triangles),
      right: formatK(right.triangles),
      better: pick(left.triangles, right.triangles),
    },
    { label: 'Texture', left: texture(left), right: texture(right), better: null },
    {
      label: 'Transfer',
      left: formatBytes(left.bytes ?? 0),
      right: formatBytes(right.bytes ?? 0),
      better: pick(Math.round((left.bytes ?? 0) / 1e4), Math.round((right.bytes ?? 0) / 1e4)),
    },
    {
      label: 'Inbox checks',
      left: checks(left),
      right: checks(right),
      better: pick(trouble(left), trouble(right)),
    },
  ];
}
