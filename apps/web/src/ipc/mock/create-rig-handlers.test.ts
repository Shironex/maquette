import { describe, expect, it, vi } from 'vitest';
import type { RigRecord } from '../client';
import { createRigHandlers } from './create-rig-handlers';

describe('createRigHandlers', () => {
  it.each([
    ['read', 'tested'],
    ['compare', 'fixed'],
    ['creature', 'testing'],
    ['refused', 'refused'],
    ['stopped', 'stopped'],
    ['one-island', 'tested'],
    ['running', 'testing'],
  ])('?rig=%s seeds a case that is %s', (state, expected) => {
    const rig = createRigHandlers(state, 0, () => undefined);
    const [record] = rig.handlers.rig_list() as RigRecord[];
    expect(record?.state).toBe(expected);
  });

  it('walks an opened model through its pose test and fix, telling the app', () => {
    vi.useFakeTimers();
    const seen: RigRecord[] = [];
    const rig = createRigHandlers(null, 1, r => seen.push(r));
    const target = { inbox: null, project: 'p', asset: 'hero', version: 1 };
    const opened = rig.handlers.rig_open({ target }) as RigRecord;
    expect(opened.state).toBe('ready');
    rig.handlers.rig_run({ id: opened.id, what: 'pose-test' });
    vi.runAllTimers();
    expect(seen.at(-1)?.state).toBe('tested');
    rig.handlers.rig_run({ id: opened.id, what: 'fix' });
    vi.runAllTimers();
    expect(seen.at(-1)?.state).toBe('fixed');
    expect(seen.at(-1)?.round).toBe(1);
    vi.useRealTimers();
  });

  it('asks a person about an agent case until it is denied', () => {
    const rig = createRigHandlers('creature', 0, () => undefined);
    expect(rig.approvals()[0]?.kind).toBe('rig-doctor');
    rig.deny('approval-rig-1');
    expect(rig.approvals()).toEqual([]);
  });
});
