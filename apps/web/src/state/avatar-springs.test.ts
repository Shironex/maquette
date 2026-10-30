import { describe, expect, it } from 'vitest';
import type { Chained, SpringCandidate } from '@/ipc/client';
import { buildNotice, specFromDefaults } from './avatar';
import {
  carriedSummary,
  chainSummary,
  confidenceLabel,
  confirmNoSprings,
  dropSpring,
  editSpring,
  lengthLabel,
  springsChoice,
  springsStaleNotice,
  toggleSpring,
  unmatchedSprings,
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

describe('what a person chose about springs', () => {
  it('tells nobody picking from confirming none, and a pick takes none back', () => {
    const spec = specFromDefaults({ author: 'Ana' }, 'Hero');
    expect(springsChoice(spec)).toBe('unpicked');
    const none = confirmNoSprings(spec, true);
    expect(springsChoice(none)).toBe('none');
    expect(none.springs).toEqual([]);
    const picked = toggleSpring(none, strip, true);
    expect(springsChoice(picked)).toBe('picked');
    expect(picked.noSprings).toBe(false);
    expect(springsChoice(confirmNoSprings(picked, true))).toBe('none');
    expect(springsChoice(toggleSpring(picked, strip, false))).toBe('unpicked');
  });

  it('finds the picks no candidate goes by now and drops them', () => {
    const old = { ...strip, id: 'cloth-1', kind: 'cloth' as const, rootBone: 'upperChest' };
    const spec = toggleSpring(specFromDefaults({ author: 'Ana' }, 'Hero'), old, true);
    expect(unmatchedSprings(spec, [strip]).map(s => s.id)).toEqual(['cloth-1']);
    expect(unmatchedSprings(spec, [old])).toEqual([]);
    expect(dropSpring(spec, 'cloth-1').springs).toEqual([]);
  });

  it('says why a build stopped for the springs', () => {
    const found = { candidates: [strip], sheet: 's.png', input: 'n.glb', height: 1.7 };
    expect(buildNotice({ status: 'candidates waiting for a pick', found }).title).toBe(
      'Pick the springs first'
    );
    const stale = buildNotice({ status: 'springs no longer match', found, stale: ['cloth-1'] });
    expect(stale.title).toBe('The confirmed springs no longer match; pick again');
    expect(stale.detail).toContain('cloth-1');
    expect(springsStaleNotice(['a', 'b']).detail).toContain('a, b');
    const carried = [{ from: 'cloth-1', to: 'hair-1', rootBone: 'head', overlap: 1 }];
    expect(carriedSummary(carried)).toBe('cloth-1 is hair-1 now');
    const waiting = buildNotice({
      status: 'springs waiting for approval',
      chained: {} as Chained,
      carried,
    });
    expect(waiting.detail).toContain('carried over: cloth-1 is hair-1 now');
  });
});
