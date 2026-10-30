import { describe, expect, it } from 'vitest';
import type { Chained, SpringCandidate } from '@/ipc/client';
import { specFromDefaults } from './avatar';
import {
  chainSummary,
  confidenceLabel,
  editSpring,
  lengthLabel,
  toggleSpring,
} from './avatar-springs';

const strip: SpringCandidate = {
  id: 'hair-1',
  kind: 'hair',
  rootBone: 'head',
  line: [
    [0, 1.6, -0.1],
    [0, 1.0, -0.2],
  ],
  length: 0.62,
  drop: 0.6,
  vertices: 792,
  confidence: 0.92,
  joints: 5,
  color: '#e8553f',
};

describe('the springs a spec confirms', () => {
  it('confirms a candidate as found, tunes it and takes it back', () => {
    const spec = specFromDefaults({ author: 'Ana' }, 'Hero');
    const on = toggleSpring(spec, strip, true);
    expect(on.springs).toEqual([
      {
        id: 'hair-1',
        kind: 'hair',
        rootBone: 'head',
        line: strip.line,
        joints: null,
        preset: null,
      },
    ]);
    const tuned = editSpring(on, 'hair-1', { joints: 7, preset: 'heavy' });
    expect(tuned.springs?.[0]).toMatchObject({ joints: 7, preset: 'heavy' });
    expect(toggleSpring(tuned, strip, false).springs).toEqual([]);
  });

  it('says a part and a chain in words', () => {
    expect(lengthLabel(1.04)).toBe('1.04 m');
    expect(lengthLabel(0.18)).toBe('18 cm');
    expect(confidenceLabel(0.917)).toBe('92%');
    const chained = {
      report: {
        chains: [{ id: 'hair-1', joints: ['a', 'b', 'c'] }],
        colliders: [{}, {}],
        changed: 8262,
      },
    } as unknown as Chained;
    expect(chainSummary(chained)).toBe(
      '1 chain (hair-1) with 3 joints, 8,262 vertices reweighted, 2 colliders from the body.'
    );
  });
});
