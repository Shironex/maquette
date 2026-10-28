import { describe, expect, it } from 'vitest';
import type { PartRow } from '@/ipc/client';
import {
  isEmptyPlan,
  mergeParts,
  planOf,
  planProblem,
  renamePart,
  toggleHidden,
  unmergePart,
} from './parts';

const rows = ['part_1', 'part_2', 'part_3'].map((id): PartRow => ({
  id,
  name: id,
  triangles: 1,
  materials: [],
  hidden: false,
  joined: [],
}));

describe('parts plan', () => {
  it('renames, and forgets a rename back to the id', () => {
    const named = renamePart({}, 'part_1', ' lid ');
    expect(named.names).toEqual({ part_1: 'lid' });
    expect(renamePart(named, 'part_1', 'part_1').names).toEqual({});
  });

  it('moves a part out of its old join', () => {
    const one = mergeParts({}, ['part_1', 'part_2']);
    const two = mergeParts(one, ['part_2', 'part_3']);
    expect(two.merge).toEqual([['part_2', 'part_3']]);
    expect(unmergePart(two, 'part_2').merge).toEqual([]);
  });

  it('hides, then shows when all are hidden', () => {
    const hidden = toggleHidden({}, ['part_1']);
    expect(hidden.hidden).toEqual(['part_1']);
    expect(toggleHidden(hidden, ['part_1']).hidden).toEqual([]);
    expect(isEmptyPlan(planOf(null))).toBe(true);
  });

  it('refuses two parts under one name, the way the core does', () => {
    expect(planProblem(rows, { names: { part_3: 'part_1' } })).toBe(
      'part_1 and part_3 would both ship as part_1.'
    );
    expect(planProblem(rows, { names: { part_1: 'x'.repeat(65) } })).toMatch(/over 64/);
    expect(planProblem(rows, { names: { part_1: 'lid' } })).toBeNull();
  });
});
