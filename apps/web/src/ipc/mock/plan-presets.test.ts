import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { madeLine, seededPlan, seedOf } from './plan-presets';

/** The core's seeds as data (`seeds.json`, kept equal to `presets.rs` by a Rust test). */
const core = JSON.parse(
  readFileSync(
    resolve(
      import.meta.dirname,
      '../../../../../crates/maquette-model/src/plans/presets/seeds.json'
    ),
    'utf8'
  )
) as Record<string, unknown>;

describe('mock presets', () => {
  it('match the core seed for seed: cards in order, places, text and links', () => {
    const now = new Date('2026-11-02T10:00:00Z').getTime();
    const keys: string[] = [];
    for (const preset of ['brainstorm', 'story', 'mechanics'] as const) {
      for (const target of ['game', 'website'] as const) {
        const { nodes, edges } = seedOf(preset, target, now);
        const at = (id: string) => nodes.findIndex(n => n.id === id);
        keys.push(`${preset}/${target}`);
        expect({
          nodes: nodes.map(n => ({ title: n.title, text: n.text, x: n.x, y: n.y })),
          links: edges.map(e => ({
            from: at(e.from),
            to: at(e.to),
            kind: e.kind,
            fromSide: e.fromSide,
            toSide: e.toSide,
          })),
        }).toEqual(core[`${preset}/${target}`]);
      }
    }
    expect(Object.keys(core).sort()).toEqual(keys.sort());
  });

  it('seed story for the target, mechanics alike for both, brainstorm blank', () => {
    const now = new Date('2026-11-02T10:00:00Z');
    const game = seededPlan('Act 1', 'story', 'game', now);
    expect(game.nodes.map(n => n.title)).toContain('Beat 1: Opening');
    expect(game.edges.filter(e => e.kind === 'leads-to')).toHaveLength(3);
    const site = seededPlan('Tour', 'story', 'website', now);
    expect(site.nodes.map(n => n.title)).toContain('Beat 1: First view');
    expect(seededPlan('Combat', 'mechanics', 'website', now).nodes).toHaveLength(4);
    const blank = seededPlan('Ideas', 'brainstorm', 'game', now);
    expect(blank.nodes).toEqual([]);
    expect(madeLine(blank)).toEqual({ ops: [], summary: 'Made the canvas Ideas' });
    const line = madeLine(game);
    expect(line.summary).toBe('Made the canvas Act 1 from the Story preset');
    expect(line.ops).toHaveLength(15);
    const ids = new Set(game.nodes.map(n => n.id));
    expect(ids.size).toBe(8);
    expect(game.edges.every(e => ids.has(e.from) && ids.has(e.to))).toBe(true);
  });
});
