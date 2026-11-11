/**
 * The rigid bind in mock mode: a winged character whose wings the auto rig
 * weighted to the arms. Binding them to the chest answers at once with the
 * bound draft measured against the fix's; only a person's answer uses it.
 */
import type { RigPart, RigRecord, RigRigid, RigRow, RigStarter } from '../client';
import { fail } from './backend-options';
import { DIR, fixedCase } from './rig-fixtures';

const PARTS: RigPart[] = [
  { part: 1, vertices: 1840, side: 'left', bones: ['LeftArm', 'LeftForeArm'], acrossCm: 41.5 },
  { part: 2, vertices: 1792, side: 'right', bones: ['RightArm', 'RightForeArm'], acrossCm: 40.8 },
];

const BODY_BONES = ['Spine2', 'Spine1', 'Spine', 'Hips', 'Neck', 'LeftShoulder', 'RightShoulder'];

/** What the bind does to a row's stretch, by pose: the wings stop following the arms. */
const BOUND_STRETCH: Record<string, number> = {
  raise: 16,
  forward: 18,
  twist: 17,
  stride: 34,
  'clip:cast_orb:0.5': 38,
  'clip:death:0.5': 41,
};

/** A fixed case with a wing on each arm and nothing bound yet. */
export function wingedCase(): RigRecord {
  const c = fixedCase('winged-imp-v1', 'Winged imp');
  return {
    ...c,
    parts: PARTS,
    bodyBones: BODY_BONES.map(b => `mixamorig:${b}`),
    rigid: null,
  };
}

/** The bound draft's row: its numbers against the fix's draft. */
function boundRow(row: RigRow): RigRow {
  const now = BOUND_STRETCH[row.id] ?? row.stretchMm;
  const better = now !== row.stretchMm;
  return {
    ...row,
    stretchMm: now,
    note: better ? `stretch ${row.stretchMm} to ${now} mm, better` : null,
    change: {
      stretchBefore: row.stretchMm,
      beyondBodyBefore: row.beyondBodyMm ?? null,
      seamGapBefore: row.seamGapMm,
      stretch: better ? 'better' : 'same',
      seamGap: 'same',
    },
  };
}

/** A bind of `parts` to `bone`, measured and waiting for a person. */
export function askedBind(c: RigRecord, parts: number[], bone: string, by: RigStarter): RigRigid {
  const rows = (c.after?.rows ?? []).map(boundRow);
  const worst = [...(c.after?.rows ?? [])].sort(
    (a, b) => (b.stretchMm ?? 0) - (a.stretchMm ?? 0)
  )[0];
  const vertices = PARTS.filter(p => parts.includes(p.part)).reduce((n, p) => n + p.vertices, 0);
  return {
    parts,
    bone,
    state: 'asked',
    askedBy: by,
    vertices,
    file: `${DIR}/${c.id}/rigid.glb`,
    earlier: c.draft,
    sheet: { file: `${DIR}/${c.id}/rigid.png`, rows, seconds: 41 },
    checks: [
      {
        ok: true,
        title: 'Only JOINTS_0 and WEIGHTS_0 changed',
        detail: 'Mesh, UVs and the texture are byte for byte the same',
        value: '2 of 14',
      },
    ],
    row: worst?.label ?? null,
    stretchBefore: worst?.stretchMm ?? null,
    stretchAfter: rows.find(r => r.id === worst?.id)?.stretchMm ?? null,
  };
}

/** The seeds: nothing bound, an agent's ask, and a bind a person used. */
export const rigidSeeds = {
  rigid: wingedCase,
  'rigid-asked': () => {
    const c = wingedCase();
    return { ...c, rigid: askedBind(c, [1, 2], 'mixamorig:Spine2', 'agent') };
  },
  'rigid-bound': () => {
    const c = wingedCase();
    const bind = askedBind(c, [1, 2], 'mixamorig:Spine2', 'person');
    return { ...c, draft: bind.file, rigid: { ...bind, state: 'approved' as const } };
  },
} satisfies Record<string, () => RigRecord>;

/** rig_bind_rigid, as the core refuses it. */
export function bindRigid(c: RigRecord, parts: number[], bone: string | null): RigRecord {
  if (c.route !== 'humanoid' || c.state !== 'fixed' || !c.draft)
    fail('invalid', 'Run the fix first: the parts are bound on its draft.');
  if (c.rigid?.state === 'approved')
    fail('invalid', "Undo the bound parts first: a new bind starts from the fix's draft.");
  if (parts.length === 0) fail('invalid', 'Pick at least one part.');
  const named = bone ?? 'Spine2';
  const whole = (c.bodyBones ?? []).find(b => b === named || b.endsWith(`:${named}`));
  if (!whole) fail('invalid', `${named} is not a body bone of this model.`);
  return { ...c, rigid: askedBind(c, parts, whole, 'person') };
}

/** rig_decide_rigid: yes uses the bound draft, no keeps the fix's, no after a yes undoes. */
export function decideRigid(c: RigRecord, approve: boolean): RigRecord {
  const bind = c.rigid;
  if (!bind) return fail('invalid', 'No bind waits for an answer.');
  if (bind.state === 'asked' && approve)
    return { ...c, draft: bind.file, rigid: { ...bind, earlier: c.draft, state: 'approved' } };
  if (bind.state === 'asked') return { ...c, rigid: { ...bind, state: 'declined' } };
  if (bind.state === 'approved' && !approve)
    return { ...c, draft: bind.earlier, rigid: { ...bind, state: 'declined' } };
  return fail('invalid', 'No bind waits for that answer.');
}
