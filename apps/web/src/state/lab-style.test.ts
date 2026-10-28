import { describe, expect, it } from 'vitest';
import { frostveilStyleBoard } from '@/ipc/mock/style-board';
import {
  EMPTY_STYLE,
  addColours,
  isStyleEmpty,
  normalizeColour,
  styleChip,
  styleCounts,
} from './lab-style';

describe('the style board helpers', () => {
  it('read colours as the core does', () => {
    expect(normalizeColour('#9FC3FF')).toBe('#9fc3ff');
    expect(normalizeColour('abc')).toBe('#aabbcc');
    expect(normalizeColour('#12345')).toBeNull();
    expect(addColours(['#111111'], '#222222, 333 #111111')).toEqual([
      '#111111',
      '#222222',
      '#333333',
    ]);
    expect(addColours([], '#222222 frost')).toBeNull();
  });

  it('count what the board holds', () => {
    expect(isStyleEmpty(EMPTY_STYLE)).toBe(true);
    expect(isStyleEmpty({ ...EMPTY_STYLE, line: 'frosted glass' })).toBe(false);
    expect(styleCounts(frostveilStyleBoard())).toBe('3 images · 4 colours');
    expect(styleCounts({ palette: ['#111111'], images: [] })).toBe('1 colour');
  });

  it('chip a direction by the revision it carried, or by going without', () => {
    const board = frostveilStyleBoard();
    expect(styleChip({ styleOff: false, styleRevision: 3 }, board)).toMatchObject({
      label: 'Style r3',
      tip: 'Made with the project style as it is now',
      on: true,
    });
    expect(styleChip({ styleOff: false, styleRevision: 2 }, board)?.tip).toBe(
      'Made with the project style at revision 2; the board is at 3 now'
    );
    expect(styleChip({ styleOff: true, styleRevision: null }, board)).toMatchObject({
      label: 'Own style',
      on: false,
    });
    expect(styleChip({ styleOff: true, styleRevision: null }, EMPTY_STYLE)).toBeNull();
    expect(styleChip({ styleOff: false, styleRevision: null }, board)).toBeNull();
  });
});
