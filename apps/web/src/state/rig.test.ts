import { describe, expect, it } from 'vitest';
import type { Check } from '@/ipc/client';
import { creatureCase } from '@/ipc/mock/rig-cases';
import { fixedCase, testedCase } from '@/ipc/mock/rig-fixtures';
import { caseLine, nextVersion, offersRigDoctor, stepLines } from './rig';

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
