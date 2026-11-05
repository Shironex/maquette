import { describe, expect, it } from 'vitest';
import { newLayer } from './recipe-draft';
import { checkLines } from './recipe-review';

describe('the preview checks in words', () => {
  it('names a gap over 2 cm and a flipped elbow or knee, and nothing else', () => {
    const hand = newLayer('reach', [], []);
    const foot = { ...newLayer('reach', [hand], []), bone: 'leftFoot' };
    const lines = checkLines(
      [
        { layer: hand.id, gap: 0.05, flipped: true },
        { layer: foot.id, gap: 0.01, flipped: false },
      ],
      [hand, foot]
    );
    expect(lines).toEqual([
      `Right hand (${hand.id}) stops 5 cm short of its target at the reach's end.`,
      `Right hand (${hand.id}): the elbow bends the other way than the clip has it.`,
    ]);
    expect(checkLines([{ layer: foot.id, gap: 0, flipped: true }], [hand, foot])[0]).toMatch(
      /knee/
    );
  });
});
