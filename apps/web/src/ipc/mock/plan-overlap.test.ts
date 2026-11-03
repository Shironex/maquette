import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { OverlapPart, PlanNode } from '../client';
import { note } from './plan-fixtures';
import { mainWords, overlapsOf, pairKey, singular } from './plan-overlap';

type Card = { id: string; title: string; text: string };
/** The core's cases (`overlap/cases.json`), which its own tests run too. */
const cases = JSON.parse(
  readFileSync(
    resolve(
      import.meta.dirname,
      '../../../../../crates/maquette-model/src/plans/overlap/cases.json'
    ),
    'utf8'
  )
) as {
  words: { part: OverlapPart; value: string; words: string[] }[];
  singular: [string, string][];
  keys: { a: Card; b: Card; key: string }[];
};

describe('the mock overlap check', () => {
  it('boils lines down as the core does', () => {
    for (const c of cases.words) expect(mainWords(c.part, c.value)).toEqual(c.words);
    for (const [word, one] of cases.singular) expect(singular(word)).toBe(one);
  });

  it('keys a pair of cards as the core does', () => {
    for (const c of cases.keys) {
      expect(pairKey(c.a, c.b)).toBe(c.key);
      expect(pairKey(c.b, c.a)).toBe(c.key);
    }
  });

  it('flags cards that share a main word, and holds a fine mark until the words change', () => {
    const card = (id: string, text: string): PlanNode => note(id, 0, 0, text, `System ${id}`);
    const nodes = [
      card('n-1', 'Verb: evade\nLook: white arcs\nInput: tap Shift'),
      card('n-2', 'Verb: slow time\nLook: pale mist dome\nInput: hold Shift'),
      card('n-3', 'Verb: what the player does most\nLook: how it reads on screen'),
      card('n-4', 'Verb: slowing\nLook: a dome'),
    ];
    const plan = { preset: 'mechanics' as const, nodes };
    expect(overlapsOf(plan)).toEqual([
      { a: 'n-1', b: 'n-2', part: 'input', word: 'shift', fine: false },
      { a: 'n-2', b: 'n-4', part: 'look', word: 'dome', fine: false },
    ]);
    expect(overlapsOf({ ...plan, preset: 'brainstorm' })).toEqual([]);
    const key = pairKey(nodes[0]!, nodes[1]!);
    const fine = [{ a: 'n-1', b: 'n-2', key }];
    expect(overlapsOf({ ...plan, fine })[0]?.fine).toBe(true);
    const edited = nodes.map(n => (n.id === 'n-2' ? { ...n, title: 'Frost veil' } : n));
    expect(overlapsOf({ ...plan, nodes: edited, fine })[0]?.fine).toBe(false);
  });

  it('stops at 500 pairs on a full canvas of look-alike cards', () => {
    const keys = Array.from({ length: 40 }, (_, k) => `k${k}`).join(', ');
    const text = `Verb: dash\nLook: rings\nInput: ${keys}`;
    const nodes = Array.from({ length: 2000 }, (_, i) =>
      note(`n-${String(i).padStart(4, '0')}`, 0, 0, text, 'System')
    );
    const started = performance.now();
    const found = overlapsOf({ preset: 'mechanics', nodes });
    expect(found).toHaveLength(500);
    expect(found[0]).toMatchObject({ a: 'n-0000', b: 'n-0001', word: 'dash' });
    expect(performance.now() - started).toBeLessThan(4000);
  });
});
