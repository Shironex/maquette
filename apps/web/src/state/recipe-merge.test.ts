import { describe, expect, it } from 'vitest';
import type { RecipeBlank } from '@/ipc/client';
import { newLayer, type RecipeDraft } from './recipe-draft';
import { mergeDrafts } from './recipe-merge';

const blank = (id: string, prop: string | null = null): RecipeBlank => ({
  id,
  label: id,
  kind: 'prop',
  prop,
  point: null,
});
const reach = newLayer('reach', [], [blank('lantern')]);
const wave = { ...newLayer('wave', [reach], []), id: 'l-2' };
const base: RecipeDraft = { layers: [reach, wave], blanks: [blank('lantern')] };

describe('merging an outside change', () => {
  it('takes what only one side touched, from either side', () => {
    const mine: RecipeDraft = {
      ...base,
      layers: [{ ...reach, settings: { ...reach.settings, weight: 0.5 } }, wave],
    };
    const theirs: RecipeDraft = {
      layers: [reach],
      blanks: [blank('lantern', 'lantern-post'), blank('shelf')],
    };
    const merged = mergeDrafts(base, mine, theirs);
    expect(merged.clashes).toEqual([]);
    expect(merged.mine.layers.map(l => [l.id, l.settings.weight])).toEqual([['l-1', 0.5]]);
    expect(merged.mine.blanks.map(b => [b.id, b.prop])).toEqual([
      ['lantern', 'lantern-post'],
      ['shelf', null],
    ]);
  });

  it('calls a change both sides made differently a clash, with each side ready', () => {
    const mine = { ...base, layers: [{ ...reach, bone: 'leftHand' }, wave] };
    const theirs = { ...base, layers: [{ ...reach, bone: 'rightFoot' }] };
    const merged = mergeDrafts(base, mine, theirs);
    expect(merged.clashes).toEqual(['change l-1']);
    expect(merged.mine.layers.map(l => l.bone)).toEqual(['leftHand']);
    expect(merged.theirs.layers.map(l => l.bone)).toEqual(['rightFoot']);
  });

  it('calls a removal on one side and an edit on the other a clash', () => {
    const mine = { ...base, blanks: [] };
    const theirs = { ...base, blanks: [blank('lantern', 'lantern-post')] };
    const merged = mergeDrafts(base, mine, theirs);
    expect(merged.clashes).toEqual(['blank lantern']);
    expect(merged.mine.blanks).toEqual([]);
    expect(merged.theirs.blanks[0]!.prop).toBe('lantern-post');
  });
});
