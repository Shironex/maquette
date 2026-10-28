import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const tokens = JSON.parse(readFileSync(resolve(__dirname, '../../tokens.json'), 'utf8'));
const raw: Record<string, string> = Object.fromEntries(
  tokens.color.tokens.map((t: { name: string; value: { dark: string } }) => [t.name, t.value.dark])
);

type Rgba = [number, number, number, number];

/** A token's colour, following `{name}` references. */
function color(name: string): Rgba {
  const value = raw[name] ?? '';
  const ref = /^\{(.+)\}$/.exec(value);
  if (ref?.[1]) return color(ref[1]);
  const rgba = /^rgba\((\d+),(\d+),(\d+),([\d.]+)\)$/.exec(value.replace(/\s/g, ''));
  if (rgba) return [Number(rgba[1]), Number(rgba[2]), Number(rgba[3]), Number(rgba[4])];
  const hex = /^#([0-9a-f]{6})$/i.exec(value);
  if (!hex?.[1]) throw new Error(`no colour for ${name}`);
  const n = parseInt(hex[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1];
}

/** Layers tints over a surface in gamma space, the way the browser paints them. */
function surface(layers: string[]): Rgba {
  return layers
    .map(color)
    .reduce((under, [r, g, b, a]) => [
      r * a + under[0] * (1 - a),
      g * a + under[1] * (1 - a),
      b * a + under[2] * (1 - a),
      1,
    ]);
}

function luminance([r, g, b]: Rgba): number {
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function ratio(fg: string, layers: string[]): number {
  const [a, b] = [luminance(surface([...layers, fg])), luminance(surface(layers))];
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

const SURFACES = [['bg-app'], ['bg-panel'], ['bg-raised'], ['bg-hover']];
/** Selected rows and items: clay-soft over the panel or a card. */
const SELECTED = [
  ['bg-panel', 'clay-soft'],
  ['bg-raised', 'clay-soft'],
];

/** [text token, the layers under it, the least ratio]. 4.5 for text, 3 for UI parts. */
const PAIRINGS: [string, string[], number][] = [
  ...['text', 'text-dim', 'text-mute'].flatMap(fg =>
    [...SURFACES, ...SELECTED].map(layers => [fg, layers, 4.5] as [string, string[], number])
  ),
  ...[...SURFACES, ...SELECTED].map(
    layers => ['clay-text', layers, 4.5] as [string, string[], number]
  ),
  // A clay chip on a selected row.
  ['clay-text', ['bg-panel', 'clay-soft', 'clay-soft'], 4.5],
  ['sage', ['bg-raised', 'sage-soft'], 4.5],
  ['sage', ['bg-panel', 'sage-soft'], 4.5],
  ['amber', ['bg-raised', 'amber-soft'], 4.5],
  ['danger', ['bg-raised', 'danger-soft'], 4.5],
  ['clay-ink', ['clay'], 4.5],
  ['clay-ink', ['clay-hover'], 4.5],
  ...SURFACES.map(layers => ['focus', layers, 3] as [string, string[], number]),
];

describe('token contrast', () => {
  it.each(PAIRINGS)('%s on %j reaches %d:1', (fg, layers, least) => {
    expect(ratio(fg, layers)).toBeGreaterThanOrEqual(least);
  });
});
