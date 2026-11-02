import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { MECHANICS, madeLine, seededPlan, STORY_GAME, STORY_WEBSITE } from './plan-presets';

const core = readFileSync(
  resolve(import.meta.dirname, '../../../../../crates/maquette-model/src/plans/presets.rs'),
  'utf8'
);

/** `text` as a Rust string literal spells it. */
const literal = (text: string) => `"${text.replaceAll('\n', '\\n')}"`;

describe('mock presets', () => {
  it('use the core words, title and text, card for card', () => {
    for (const card of [...STORY_GAME, ...STORY_WEBSITE, ...MECHANICS]) {
      expect(core).toContain(literal(card.title));
      expect(core).toContain(literal(card.text));
    }
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
