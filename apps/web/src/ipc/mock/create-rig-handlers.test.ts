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

  it('seeds a creature draft that waits for its chains, and refuses to land it until then', () => {
    const rig = createRigHandlers('creature-fixed', 0, () => undefined);
    const [c] = rig.handlers.rig_list() as RigRecord[];
    expect(c?.state).toBe('fixed');
    expect(() => rig.handlers.rig_land({ id: c!.id })).toThrow(/chains/);
    const seen = rig.handlers.rig_confirm_chains({ id: c!.id, edits: [] }) as RigRecord;
    expect(seen.state).toBe('fixed');
    expect(seen.chainsConfirmed).toBe(true);
    expect((rig.handlers.rig_land({ id: c!.id }) as RigRecord).state).toBe('landed');
  });

  it('starts a case over when a chain changes, and keeps the edit through the runs', () => {
    vi.useFakeTimers();
    const rig = createRigHandlers('creature-fixed', 1, () => undefined);
    const [c] = rig.handlers.rig_list() as RigRecord[];
    const edits = [{ key: 'bone_20', readAs: 'junk', side: null }];
    const over = rig.handlers.rig_confirm_chains({ id: c!.id, edits }) as RigRecord;
    expect(over.state).toBe('ready');
    expect(over.draft).toBeNull();
    expect(over.creature?.chains.find(r => r.key === 'bone_20')?.readAs).toBe('junk');
    rig.handlers.rig_run({ id: c!.id, what: 'pose-test' });
    vi.runAllTimers();
    const tested = rig.handlers.rig_get({ id: c!.id }) as RigRecord;
    expect(tested.route).toBe('creature');
    expect(tested.chainEdits).toEqual(edits);
    rig.handlers.rig_run({ id: c!.id, what: 'fix' });
    vi.runAllTimers();
    rig.handlers.rig_run({ id: c!.id, what: 'walk-proof' });
    vi.runAllTimers();
    const walked = rig.handlers.rig_get({ id: c!.id }) as RigRecord;
    expect(walked.state).toBe('fixed');
    expect(walked.walkProof).toContain('walk_proof');
    expect(walked.round).toBe(1);
    vi.useRealTimers();
  });

  it('turns a case to the other route only as the core would', () => {
    const rig = createRigHandlers('read', 0, () => undefined);
    const [c] = rig.handlers.rig_list() as RigRecord[];
    const turned = rig.handlers.rig_set_route({ id: c!.id, route: 'creature' }) as RigRecord;
    expect(turned.route).toBe('creature');
    expect(turned.before).toBeNull();
    const own = createRigHandlers('creature-fixed', 0, () => undefined);
    const [hound] = own.handlers.rig_list() as RigRecord[];
    expect(() => own.handlers.rig_set_route({ id: hound!.id, route: 'humanoid' })).toThrow(/draft/);
  });

  it('asks a person about an agent case until it is denied', () => {
    const rig = createRigHandlers('creature', 0, () => undefined);
    expect(rig.approvals()[0]?.kind).toBe('rig-doctor');
    rig.deny('approval-rig-1');
    expect(rig.approvals()).toEqual([]);
  });
});
