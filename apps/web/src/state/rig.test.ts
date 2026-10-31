import { describe, expect, it } from 'vitest';
import type { Check } from '@/ipc/client';
import { creatureCase } from '@/ipc/mock/rig-cases';
import { fixedCase, testedCase } from '@/ipc/mock/rig-fixtures';
import { creatureFixedCase } from '@/ipc/mock/rig-cases';
import {
  addWaitsFor,
  caseLine,
  chainChanged,
  nextVersion,
  offersRigDoctor,
  rigStatusLine,
  routeHeldBy,
  stepLines,
} from './rig';

const check = (id: string, status: Check['status']): Check => ({
  id,
  status,
  title: '',
  detail: null,
  fixes: [],
});

describe('rig words', () => {
  it('offers the doctor on the rig checks that did not pass', () => {
    expect(offersRigDoctor(check('finger-weights', 'fail'))).toBe(true);
    expect(offersRigDoctor(check('arm-weights', 'warning'))).toBe(true);
    expect(offersRigDoctor(check('rig', 'warning'))).toBe(true);
    expect(offersRigDoctor(check('rig', 'pass'))).toBe(false);
    expect(offersRigDoctor(check('triangles', 'fail'))).toBe(false);
  });

  it('marks the steps as a case stands and names the next version', () => {
    const lines = stepLines(testedCase('a-v1', 'A'));
    expect(lines.map(l => l.mark)).toEqual(['done', 'done', 'on', 'todo', 'todo', 'todo']);
    expect(lines[2]?.detail).toBe('9 cells, 3 defects found');
    const fixed = fixedCase('a-v1', 'A');
    expect(nextVersion(fixed)).toBe(2);
    expect(stepLines(fixed)[5]).toMatchObject({
      title: 'Land as v2',
      detail: 'Waits for Add as v2',
    });
  });

  it('says how a case stands in the Inbox', () => {
    expect(caseLine(creatureCase())).toBe('Spiked hound: pose test after 9 of 13');
    expect(caseLine(fixedCase('a-v1', 'A'))).toBe('A: draft v2 waits');
  });
});

describe('what a person still decides', () => {
  it('says what a running case is at, for the status bar', () => {
    expect(rigStatusLine([])).toBeNull();
    expect(rigStatusLine([testedCase('a', 'A')])).toBeNull();
    expect(rigStatusLine([testedCase('a', 'A'), creatureCase()])).toBe(
      'Rig doctor: pose test after 9 of 13'
    );
    expect(rigStatusLine([{ ...creatureCase(), progress: null }])).toBe(
      'Rig doctor: Spiked hound running'
    );
  });

  it('holds Add until a creatures chains were looked at', () => {
    expect(addWaitsFor(fixedCase('a', 'A'))).toBeNull();
    expect(addWaitsFor(creatureFixedCase())).toMatch(/Look at the chains/);
    expect(addWaitsFor({ ...creatureFixedCase(), chainsConfirmed: true })).toBeNull();
  });

  it('holds the route on a run, a draft and a landed case', () => {
    expect(routeHeldBy(testedCase('a', 'A'))).toBeNull();
    expect(routeHeldBy(creatureCase())).toMatch(/run/);
    expect(routeHeldBy(fixedCase('a', 'A'))).toMatch(/draft/);
    expect(routeHeldBy({ ...testedCase('a', 'A'), state: 'landed' })).toMatch(/version/);
  });

  it('sees through an edit that says what the table says', () => {
    const rows = creatureCase().creature!.chains;
    expect(chainChanged({ key: 'bone_1', readAs: 'chain', side: 'left' }, rows)).toBe(false);
    expect(chainChanged({ key: 'bone_1', readAs: null, side: 'right' }, rows)).toBe(true);
    expect(chainChanged({ key: 'bone_1', readAs: 'junk', side: null }, rows)).toBe(true);
    expect(chainChanged({ key: 'dup_FL', readAs: 'chain', side: null }, rows)).toBe(true);
  });
});
