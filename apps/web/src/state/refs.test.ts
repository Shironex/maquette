import { describe, expect, it } from 'vitest';
import type { Asset, AssetRef, Direction } from '@/ipc/client';
import {
  assign,
  candidatesByView,
  emptyViews,
  sendOrder,
  sentView,
  viewsLine,
  warningsLine,
} from './refs';

const refs: AssetRef[] = [
  { file: 'refs/lantern.png', view: 'front' },
  { file: 'refs/lantern_side.png', view: 'left' },
  { file: 'refs/lantern_kit.png', view: 'other' },
];

const asset = (tags: AssetRef[]) => ({ id: 'frost-lantern', refs: tags }) as unknown as Asset;

describe('ref views', () => {
  it('swaps the two refs when a taken view is picked', () => {
    const next = assign(refs, 'refs/lantern_kit.png', 'left');
    expect(next).toEqual([
      { file: 'refs/lantern.png', view: 'front' },
      { file: 'refs/lantern_side.png', view: 'other' },
      { file: 'refs/lantern_kit.png', view: 'left' },
    ]);
    expect(assign(refs, 'refs/lantern.png', 'left').map(r => r.view)).toEqual([
      'left',
      'front',
      'other',
    ]);
  });

  it('adds a new ref with its view, and the old holder becomes other', () => {
    const next = assign(refs, 'refs/lantern_back.png', 'front');
    expect(next.find(r => r.file === 'refs/lantern.png')?.view).toBe('other');
    expect(next.at(-1)).toEqual({ file: 'refs/lantern_back.png', view: 'front' });
    expect(assign(refs, 'refs/new.png', 'other')).toHaveLength(4);
  });

  it('counts the filled views and names the empty ones', () => {
    expect(viewsLine(refs)).toBe('2 of 4 views');
    expect(emptyViews(refs)).toEqual(['back', 'right']);
  });

  it('sends tagged images front first and leaves other refs home', () => {
    const picked = ['refs/lantern_kit.png', 'refs/lantern_side.png', 'refs/lantern.png'];
    expect(sendOrder(asset(refs), picked)).toEqual(['refs/lantern.png', 'refs/lantern_side.png']);
    expect(sentView(asset(refs), picked, 'refs/lantern_kit.png')).toBeNull();
  });

  it('sends an untagged asset in the order picked', () => {
    const picked = ['refs/b.png', 'refs/a.png'];
    expect(sendOrder(asset([]), picked)).toEqual(picked);
    expect(sentView(asset([]), picked, 'refs/a.png')).toBe('left');
  });

  it('groups the drawn candidates by view, with the one in its slot marked', () => {
    const direction = {
      id: 'frost-lantern-views',
      refsJob: { asset: 'frost-lantern', kind: 'views' },
      images: [
        { file: 'concepts/v/v_01.png', title: 'back', refFile: null },
        { file: 'concepts/v/v_02.png', title: 'left', refFile: 'refs/lantern_side.png' },
      ],
    } as unknown as Direction;
    const jobs = [{ direction: direction.id, title: 'right', state: 'running' }] as never;
    const grouped = candidatesByView(refs, [direction], jobs);
    expect(grouped.left.images[0]?.inSlot).toBe(true);
    expect(grouped.back.images.map(c => c.file)).toEqual(['concepts/v/v_01.png']);
    expect(grouped.right.drawing).toBe(1);
  });

  it('counts warnings in words', () => {
    expect(warningsLine(1)).toBe('1 ref warning');
    expect(warningsLine(3)).toBe('3 ref warnings');
  });
});
