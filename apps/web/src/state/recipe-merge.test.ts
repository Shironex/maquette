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
const noBase = { clips: [], kept: [], time: [], mirror: false };
const base: RecipeDraft = { layers: [reach, wave], blanks: [blank('lantern')], ...noBase };

describe('merging an outside change', () => {
  it('takes what only one side touched, from either side', () => {
    const mine: RecipeDraft = {
      ...base,
      layers: [{ ...reach, settings: { ...reach.settings, weight: 0.5 } }, wave],
    };
    const theirs: RecipeDraft = {
      ...noBase,
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
    const mine = { ...noBase, layers: [...base.layers, mineAim], blanks: [...base.blanks, torch] };
    const theirsAim = { ...wave, id: 'l-k3x9q' };
    const theirs = {
      ...noBase,
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
    expect(merged.renamed).toEqual({
      blank: { 'prop-k3x9q': renamed?.id },
      layer: { 'l-k3x9q': aim?.id },
    });
  });

  it('takes an id both sides added with the same entry once', () => {
    const torch = blank('prop-k3x9q', 'torch');
    const both = { ...base, blanks: [...base.blanks, torch] };
    const merged = mergeDrafts(base, both, both);
    expect(merged.clashes).toEqual([]);
    expect(merged.mine.blanks.map(b => b.id)).toEqual(['lantern', 'prop-k3x9q']);
  });

  it('merges the clips and the mirror field by field, clips and time edits as one', () => {
    const a = { name: 'A', frames: 30, length: 1, hash: 'a', mixamo: false };
    const b = { ...a, name: 'B', hash: 'b', blend: { frames: 8 } };
    const clipped = { ...base, clips: [a, b], kept: [0, 1] };
    const mine = { ...clipped, mirror: true };
    const theirs = { ...clipped, time: [{ kind: 'freeze' as const, clip: 1, at: 4, frames: 3 }] };
    const merged = mergeDrafts(clipped, mine, theirs);
    expect(merged.clashes).toEqual([]);
    expect(merged.mine).toMatchObject({ mirror: true, time: theirs.time });
    const moved = { ...clipped, clips: [b, a], kept: [1, 0] };
    const clash = mergeDrafts(clipped, moved, theirs);
    expect(clash.clashes).toEqual(['the clips and time edits']);
    expect(clash.mine.clips.map(c => c.name)).toEqual(['B', 'A']);
    expect(clash.theirs.time).toEqual(theirs.time);
  });
});
