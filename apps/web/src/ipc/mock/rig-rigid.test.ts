import { describe, expect, it } from 'vitest';
import type { RigRecord } from '../client';
import { createRigHandlers } from './create-rig-handlers';
import { refuseHeldJointMove } from './rig-rigid';

const seeded = (state: string) => {
  const rig = createRigHandlers(state, 0, () => undefined);
  const [c] = rig.handlers.rig_list() as RigRecord[];
  return { h: rig.handlers, c: c! };
};

describe('the rigid bind in mock mode', () => {
  it('binds the wings to the chest by default and measures it against the draft', () => {
    const { h, c } = seeded('rigid');
    const asked = h.rig_bind_rigid({ id: c.id, parts: [1, 2], bone: null }) as RigRecord;
    expect(asked.rigid?.state).toBe('asked');
    expect(asked.rigid?.bone).toBe('mixamorig:Spine2');
    expect(asked.rigid?.stretchAfter).toBeLessThan(asked.rigid?.stretchBefore ?? 0);
    expect(asked.draft).toBe(c.draft);
  });

  it('refuses a bone that is not a body bone, and a bind with no part', () => {
    const { h, c } = seeded('rigid');
    expect(() => h.rig_bind_rigid({ id: c.id, parts: [1], bone: 'LeftArm' })).toThrow();
    expect(() => h.rig_bind_rigid({ id: c.id, parts: [], bone: null })).toThrow();
  });

  it('uses the bound draft on a yes and puts the fix draft back on undo', () => {
    const { h, c } = seeded('rigid-asked');
    const used = h.rig_decide_rigid({ id: c.id, approve: true }) as RigRecord;
    expect(used.rigid?.state).toBe('approved');
    expect(used.draft).toBe(c.rigid?.file);
    const undone = h.rig_decide_rigid({ id: c.id, approve: false }) as RigRecord;
    expect(undone.rigid?.state).toBe('declined');
    expect(undone.draft).toBe(c.draft);
  });

  it('keeps a bind and a joint move from both landing, in either order', () => {
    const held = seeded('rigid-held');
    expect(() => held.h.rig_bind_rigid({ id: held.c.id, parts: [1], bone: null })).toThrow();
    const bound = seeded('rigid-bound');
    const withMove = { ...bound.c, jointMove: seeded('rigid-held').c.jointMove };
    expect(() => refuseHeldJointMove(withMove, true)).toThrow();
    expect(() => refuseHeldJointMove(withMove, false)).not.toThrow();
  });
});
