import { describe, expect, it } from 'vitest';
import {
  DEFAULT_TRIPO,
  assembleProblem,
  confirmLine,
  meshyLine,
  meshyNames,
  meshyProblem,
  movePart,
  nodeName,
  splitCredits,
  type DraftPart,
} from './split';

const PRICES = { tripoSegment: 40, tripoComplete: 100, meshySplit: 10 };

const part = (name: string): DraftPart => ({
  asset: name,
  version: null,
  name,
  pivot: 'bottom-centre',
  at: null,
  turn: 0,
  triangles: 100,
});

describe('split', () => {
  it('prices each way from the published numbers', () => {
    expect(splitCredits('blender', DEFAULT_TRIPO, PRICES)).toBe(0);
    expect(splitCredits('tripo', DEFAULT_TRIPO, PRICES)).toBe(40);
    expect(splitCredits('tripo', { ...DEFAULT_TRIPO, complete: 'quick-cap' }, PRICES)).toBe(140);
    expect(splitCredits('meshy', DEFAULT_TRIPO, PRICES)).toBe(10);
  });

  it('asks before credits go out, with the cancel rule', () => {
    const semantic = { ...DEFAULT_TRIPO, model: 'semantic' as const };
    expect(confirmLine('tripo', semantic, { mode: 'auto' }, PRICES, 1)).toBe(
      '1 segment task, semantic model, balanced. About 40 credits. Tripo cannot cancel once it is sent.'
    );
    const named = { mode: 'by-parts' as const, parts: ['lid', 'cage'] };
    expect(confirmLine('meshy', DEFAULT_TRIPO, named, PRICES, 1)).toBe(
      'Meshy Auto Split, by parts, 2 names, then a bake back in Blender. About 10 credits. You can cancel while Meshy has not started.'
    );
  });

  it('reads Meshy part names the way the core sends them', () => {
    expect(meshyNames([' lid ', 'Lid', '', 'cage'])).toEqual(['lid', 'cage']);
    expect(meshyProblem({ mode: 'auto', parts: [] })).toBeNull();
    expect(meshyProblem({ mode: 'by-parts', parts: [' '] })).toBe(
      'Name at least one part for Meshy to look for.'
    );
    const many = Array.from({ length: 11 }, (_, i) => `part${i}`);
    expect(meshyProblem({ mode: 'by-color', parts: many })).toBe(
      'Meshy looks for at most 10 parts.'
    );
    expect(meshyLine({ mode: 'by-color', parts: ['ice'] })).toBe(
      'Meshy Auto Split, by colour, 1 name'
    );
  });

  it('checks an assembly like the core does', () => {
    expect(assembleProblem('', [part('a'), part('b')])).toBe(
      'Give the prop a name of 1 to 64 characters.'
    );
    expect(assembleProblem('sigil_gate', [part('frame')])).toBe('Assemble 2 to 16 parts, not 1.');
    expect(assembleProblem('sigil_gate', [part('frame'), part('frame')])).toBe(
      'Two parts are named frame. Give each its own name.'
    );
    expect(assembleProblem('sigil_gate', [part('frame'), part('sigil_gate')])).toBe(
      "sigil_gate is the prop's own name. Give the part another."
    );
    expect(assembleProblem('sigil_gate', [part('frame'), part('door_leaf')])).toBeNull();
  });

  it('names nodes and moves parts in the list', () => {
    expect(nodeName('Sigil gate door')).toBe('sigil_gate_door');
    expect(nodeName('***')).toBe('part');
    expect(movePart(['a', 'b', 'c'], 2, -1)).toEqual(['a', 'c', 'b']);
    expect(movePart(['a', 'b'], 0, -1)).toEqual(['a', 'b']);
  });
});
