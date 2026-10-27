import { describe, expect, it, vi } from 'vitest';
import type { AssetKind, InspectView } from '@/ipc/client';
import type { SidePick } from '@/state/inspector';
import { compareGroups, filterGroups, pickable } from './compare-options';

const side = (id: string, label: string) => ({ id, label, note: null, members: [] });
const asset = (id: string, label: string, versions: number) => ({
  id: `asset/${id}`,
  label,
  kind: 'asset' as const,
  sides: Array.from({ length: versions }, (_, i) => side(`${id}${i}`, `v${i + 1}`)),
});

const scene = { id: 'scene', label: 'Scene', kind: 'scene' as const, sides: [side('a', 'v1')] };
const view = {
  comparisons: [
    scene,
    asset('brazier', 'Brazier', 3),
    asset('armillary', 'armillary sphere', 1),
    asset('keeper', 'Elsheria showcase', 2),
    asset('stray', 'Stray', 1),
  ],
} as unknown as InspectView;
const pick = { comparison: scene, right: scene.sides[0], left: null } as unknown as SidePick;
const kinds = new Map<string, AssetKind>([
  ['brazier', 'prop'],
  ['armillary', 'prop'],
  ['keeper', 'character'],
]);

describe('compareGroups', () => {
  const groups = compareGroups(view, pick, kinds, vi.fn());

  it('groups assets by kind, characters first, and the rest under Other', () => {
    expect(groups.map(g => g.title)).toEqual(['Scene', 'Characters', 'Props', 'Other']);
  });

  it('sorts a group by name without regard to case', () => {
    const props = groups.find(g => g.title === 'Props')!;
    expect(props.options.map(o => o.label)).toEqual(['armillary sphere', 'Brazier']);
  });

  it('shows the version count once, as a trailing note', () => {
    const props = groups.find(g => g.title === 'Props')!;
    expect(props.options.map(o => o.detail)).toEqual(['1 version', '3 versions']);
  });

  it('marks the open comparison, which is not pickable', () => {
    expect(pickable(groups).some(o => o.current)).toBe(false);
  });
});

describe('filterGroups', () => {
  it('keeps the matches in any case and drops empty groups', () => {
    const groups = compareGroups(view, pick, kinds, vi.fn());
    const found = filterGroups(groups, 'ELSH');
    expect(found.map(g => g.title)).toEqual(['Characters']);
  });
});
