import { describe, expect, it } from 'vitest';
import type { RecipeBlank } from '@/ipc/client';
import {
  blankFill,
  frameOf,
  freeId,
  layerLine,
  newBlank,
  newLayer,
  shareOf,
  withBlankKind,
  withKind,
  withoutBlank,
} from './recipe-draft';

const lantern: RecipeBlank = {
  id: 'lantern',
  label: 'Lantern',
  kind: 'prop',
  prop: 'lantern',
  point: null,
};

describe('recipe drafts', () => {
  it('starts a change on its kind with what the kind reads and a free id', () => {
    const reach = newLayer('reach', [], [lantern]);
    expect(reach).toMatchObject({ id: 'l-1', bone: 'rightHand' });
    expect(reach.settings).toMatchObject({
      target: 'lantern',
      start: 0.25,
      end: 0.75,
      rotation: null,
    });
    const wave = newLayer('wave', [reach], []);
    expect(wave.id).toBe('l-2');
    expect(wave.settings).toMatchObject({ cycles: 3, amount: 30, target: null });
    expect(newLayer('nudge', [], []).settings.rotation).toEqual([0, 0, 0]);
  });

  it('keeps the bone and range on a kind switch when the new kind may use them', () => {
    const reach = newLayer('reach', [], [lantern]);
    const wave = withKind(reach, 'wave', [lantern]);
    expect(wave.bone).toBe('rightHand');
    expect(wave.settings).toMatchObject({ start: 0.25, target: null, cycles: 3 });
    expect(withKind(reach, 'look-at', [lantern]).bone).toBe('head');
  });

  it('adds open blanks, switches their kind and removes one with what aimed at it', () => {
    const point = newBlank('point', [lantern]);
    expect(point).toMatchObject({ id: 'point-1', kind: 'point', point: null });
    expect(withBlankKind(lantern, 'point')).toMatchObject({ prop: null, point: null });
    const reach = newLayer('reach', [], [lantern]);
    const left = withoutBlank({ layers: [reach], blanks: [lantern, point] }, 'lantern');
    expect(left.blanks).toEqual([point]);
    expect(left.layers[0]!.settings.target).toBeNull();
    expect(freeId('l', ['l-1', 'l-3'])).toBe('l-2');
  });

  it('turns shares into frames and says each change and fill in words', () => {
    expect(frameOf(0.25, 120)).toBe(30);
    expect(shareOf(150, 120)).toBe(1);
    expect(layerLine(newLayer('reach', [], [lantern]), 120)).toBe(
      'Reach · Right hand → lantern · frames 30 to 90'
    );
    expect(blankFill(lantern, () => 'Lantern post')).toBe('Lantern post');
    expect(blankFill({ ...lantern, prop: null }, () => null)).toBe('open, pick a prop');
    expect(
      blankFill({ ...lantern, kind: 'point', prop: null, point: [0.5, 1, 0] }, () => null)
    ).toBe('0.50, 1.00, 0.00 m');
  });
});
