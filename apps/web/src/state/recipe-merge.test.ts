import { describe, expect, it } from 'vitest';
import type { RecipeBlank } from '@/ipc/client';
import { addClip } from './recipe-clips';
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
    const clipped = { ...base, layers: [], clips: [a, b], kept: [0, 1] };
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

describe('merging changes read on other clips', () => {
  const clip = (name: string, frames = 60) => ({
    name,
    frames,
    length: 2,
    hash: name,
    mixamo: false,
  });
  const span = (id: string, start: number, end: number) => ({
    ...wave,
    id,
    settings: { ...wave.settings, start, end },
  });
  // One 60 frame clip with a change over its frames 30 to 48.
  const one: RecipeDraft = {
    ...noBase,
    layers: [span('l-1', 0.5, 0.8)],
    blanks: [],
    clips: [clip('Idle')],
    kept: [0],
  };
  const shares = (d: RecipeDraft) => d.layers.map(l => [l.id, l.settings.start, l.settings.end]);

  it("moves theirs' edit onto the clip mine joined", () => {
    const mine = addClip(one, clip('Wave'));
    const theirs = {
      ...one,
      layers: [
        { ...span('l-1', 0.5, 0.8), settings: { ...span('l-1', 0.5, 0.8).settings, weight: 0.3 } },
      ],
    };
    const merged = mergeDrafts(one, mine, theirs);
    expect(merged.clashes).toEqual([]);
    expect(merged.mine.layers[0]?.settings).toMatchObject({ start: 0.25, end: 0.4, weight: 0.3 });
  });

  it('moves a change mine added onto the clip theirs joined in front', () => {
    const mine = { ...one, layers: [...one.layers, span('l-k3x9q', 0.1, 0.2)] };
    // The core moved theirs' l-1 when it joined Wave before Idle.
    const theirs = {
      ...one,
      clips: [clip('Wave'), { ...clip('Idle'), blend: { frames: 8 } }],
      kept: [0, 1],
      layers: [span('l-1', 0.75, 0.9)],
    };
    const merged = mergeDrafts(one, mine, theirs);
    expect(merged.clashes).toEqual([]);
    expect(shares(merged.mine)).toEqual([
      ['l-1', 0.75, 0.9],
      ['l-k3x9q', 0.55, 0.6],
    ]);
  });

  it('calls a change on a clip mine took out a clash, with the clips', () => {
    const two = {
      ...one,
      layers: [],
      clips: [clip('Idle'), { ...clip('Wave'), blend: { frames: 8 } }],
      kept: [0, 1],
    };
    const mine = { ...two, clips: [clip('Idle')], kept: [0] };
    const theirs = { ...two, layers: [span('l-2', 0.6, 0.7)] };
    const merged = mergeDrafts(two, mine, theirs);
    expect(merged.clashes).toEqual(['change l-2', 'the clips and time edits']);
    expect(merged.mine.layers).toEqual([]);
    expect(merged.theirs.clips).toHaveLength(2);
    expect(shares(merged.theirs)).toEqual([['l-2', 0.6, 0.7]]);
  });
});
