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
const reach = { ...newLayer('reach', [], [blank('lantern')]), id: 'l-1' };
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

  it('keeps an id both sides added with different entries as two additions, never a clash', () => {
    const torch = { ...blank('prop-k3x9q', 'torch'), label: 'Mine' };
    const mineAim = { ...reach, id: 'l-k3x9q', settings: { ...reach.settings, target: torch.id } };
    const mine = { layers: [...base.layers, mineAim], blanks: [...base.blanks, torch] };
    const theirsAim = { ...wave, id: 'l-k3x9q' };
    const theirs = {
      layers: [...base.layers, theirsAim],
      blanks: [...base.blanks, { ...torch, label: 'Theirs', prop: 'barrel' }],
    };
    const merged = mergeDrafts(base, mine, theirs);
    expect(merged.clashes).toEqual([]);
    expect(merged.mine).toEqual(merged.theirs);
    const ids = merged.mine.blanks.map(b => b.id);
    expect(ids).toHaveLength(3);
    expect(merged.mine.blanks.find(b => b.id === 'prop-k3x9q')?.label).toBe('Theirs');
    const renamed = merged.mine.blanks.find(b => b.label === 'Mine');
    expect(renamed?.id).toMatch(/^prop-[a-z0-9]{5}$/);
    expect(renamed?.id).not.toBe('prop-k3x9q');
    expect(merged.mine.layers).toHaveLength(4);
    const aim = merged.mine.layers.find(l => l.kind === 'reach' && l.id !== 'l-1');
    expect(aim?.id).not.toBe('l-k3x9q');
    expect(aim?.settings.target).toBe(renamed?.id);
  });

  it('takes an id both sides added with the same entry once', () => {
    const torch = blank('prop-k3x9q', 'torch');
    const both = { ...base, blanks: [...base.blanks, torch] };
    const merged = mergeDrafts(base, both, both);
    expect(merged.clashes).toEqual([]);
    expect(merged.mine.blanks.map(b => b.id)).toEqual(['lantern', 'prop-k3x9q']);
  });
});
