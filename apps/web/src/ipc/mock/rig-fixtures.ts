/**
 * Rig doctor cases in mock mode, one per board and stop. The pictures are
 * drawn SVG figures, so no model or render is bundled.
 */
import type { RigRecord, RigRow, RigSheet } from '../client';
import { registerMockFile } from '../files';

export const DIR = '~/Maquette/cache/rig';

/** Raised arms per pose, and whether something swings wrongly. */
const LIFT: Record<string, number> = { raise: -40, forward: -8, hand: 18, stride: 22 };

/** A little figure on a studio ground: `wrong` drags a cape out in the clay colour. */
export function figure(pose: string, _view: string, wrong: boolean, moved = false): string {
  const lift = LIFT[pose] ?? 22;
  const armY = 44 + lift;
  const legs =
    pose === 'stride' ? 'M50 72 L38 104 M50 72 L62 104' : 'M46 72 L44 104 M54 72 L56 104';
  const cape = wrong
    ? `<path d="M40 36 L${18} ${armY + 6} L38 78 Z" fill="#d67f5f" opacity=".85"/>`
    : '<path d="M42 36 L36 80 L64 80 L58 36 Z" fill="#7d8aa0" opacity=".7"/>';
  const left = moved ? 26 : 30;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -11 100 133"><rect y="-11" width="100" height="133" fill="#312f2b"/>${cape}<circle cx="50" cy="24" r="8" fill="#8e8a84"/><path d="M50 32 L50 72" stroke="#8e8a84" stroke-width="9" stroke-linecap="round"/><path d="M50 38 L${left} ${armY} M50 38 L72 ${armY}" stroke="#8e8a84" stroke-width="5" stroke-linecap="round"/><path d="${legs}" stroke="#8e8a84" stroke-width="6" stroke-linecap="round"/></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function picture(path: string, pose: string, view: string, wrong: boolean, moved = false): string {
  registerMockFile(path, figure(pose, view, wrong, moved));
  return path;
}

/** The numbers of a row, in mm: stretch, past the body, seam gap. */
export type RowNumbers = readonly [number, number, number];

export interface RowPlan {
  id: string;
  note?: string;
  look?: string;
  worst?: number;
  defects?: number[];
  before: RowNumbers;
  after?: RowNumbers;
}

export const HUMAN_ROWS: RowPlan[] = [
  { id: 'rest', before: [0, 0, 0], after: [0, 0, 0] },
  {
    id: 'raise',
    note: 'cloth swings',
    defects: [1, 3],
    before: [62, 48, 310],
    after: [57, 31, 0],
  },
  { id: 'forward', note: 'cloth swings', defects: [1], before: [76, 52, 359], after: [66, 29, 0] },
  { id: 'hand', note: 'hair follows', defects: [2], before: [244, 236, 45], after: [12, 12, 0] },
  { id: 'twist', note: 'seam opens 112 mm', before: [24, 20, 112], after: [28, 21, 0] },
  {
    id: 'head',
    look: 'a few edges stretch 92 mm near Neck, look here',
    worst: 92,
    before: [108, 18, 0],
    after: [12, 10, 0],
  },
  { id: 'stride', note: 'seam opens 54 mm', before: [136, 90, 54], after: [98, 33, 0] },
  {
    id: 'clip:cast_orb:0.5',
    note: 'seam opens 393 mm',
    before: [145, 96, 393],
    after: [220, 41, 0],
  },
  {
    id: 'clip:death:0.5',
    note: 'seam opens 141 mm',
    before: [173, 104, 141],
    after: [275, 57, 0],
  },
];

const SAME_MM = 5;
const trend = (was: number, now: number) =>
  now - was >= SAME_MM ? 'worse' : was - now >= SAME_MM ? 'better' : 'same';

function rows(tag: string, plan: RowPlan[], fixed: boolean, moved = false): RigRow[] {
  return plan.map(p => {
    const wrong = !fixed && Boolean(p.defects?.length);
    const cell = (view: string) =>
      picture(`${DIR}/${tag}/${p.id}_${view}.svg`, p.id, view, wrong, moved);
    const now = fixed ? (p.after ?? p.before) : p.before;
    const stretch = trend(p.before[0], now[0]);
    const seam = trend(p.before[2], now[2]);
    const said = [
      stretch === 'same' ? null : `stretch ${p.before[0]} to ${now[0]} mm, ${stretch}`,
      seam === 'same' ? null : `seam ${p.before[2]} to ${now[2]} mm, ${seam}`,
    ].filter(Boolean);
    return {
      id: p.id,
      label: p.id.replace('clip:', '').replace(':', ' '),
      note: fixed ? said.join('; ') || null : (p.note ?? null),
      defects: fixed ? [] : (p.defects ?? []),
      stretchMm: now[0],
      beyondBodyMm: now[1],
      worstEdgeMm: p.worst ?? now[1],
      look: p.look ?? null,
      seamGapMm: now[2],
      front: cell('front'),
      side: cell('side'),
      back: fixed ? null : cell('back'),
      xray: fixed ? null : cell('xray'),
      beforeFront: fixed
        ? picture(`${DIR}/${tag}/${p.id}_was_front.svg`, p.id, 'front', Boolean(p.defects?.length))
        : null,
      beforeSide: fixed
        ? picture(`${DIR}/${tag}/${p.id}_was_side.svg`, p.id, 'side', Boolean(p.defects?.length))
        : null,
      change: fixed
        ? {
            stretchBefore: p.before[0],
            beyondBodyBefore: p.before[1],
            seamGapBefore: p.before[2],
            stretch,
            seamGap: seam,
          }
        : null,
    };
  });
}

export const sheet = (tag: string, plan: RowPlan[], fixed: boolean, moved = false): RigSheet => ({
  file: picture(`${DIR}/${tag}/sheet.svg`, 'sheet', tag, !fixed, moved),
  rows: rows(tag, plan, fixed, moved),
  seconds: 38,
});

/** A case that has been read and nothing else. */
export function readCase(id: string, name: string, inbox: string | null): RigRecord {
  return {
    id,
    name,
    startedBy: 'person',
    route: 'humanoid',
    state: 'ready',
    step: inbox ? 'accept' : 'before',
    source: inbox
      ? { inbox, project: null, asset: null, version: null }
      : { inbox: null, project: 'frostveil', asset: 'duchess-vesperine', version: 1 },
    input: `~/Maquette/inbox/${id}.glb`,
    dir: `${DIR}/${id}`,
    model: {
      rig: 'Mixamo, 65 joints',
      joints: 65,
      skinnedMeshes: 1,
      clips: ['idle', 'walk'],
      triangles: 96_400,
      height: 1.72,
      textures: 1,
      handFar: 1284,
      armFar: 3912,
      islands: 12,
      oneIsland: false,
      humanoidOk: true,
      finding:
        'Hair vertices weighted to finger bones. 1,284 vertices far from the hands follow LeftHandIndex1 (652) and RightHandIndex1 (632).',
    },
    issues: [],
    creature: null,
    before: null,
    after: null,
    defects: [],
    stats: [],
    fixSteps: [],
    checks: [],
    jointMove: null,
    round: 0,
    draft: null,
    progress: null,
    stages: [],
    log: [],
    stop: null,
    chainsConfirmed: false,
    chainEdits: [],
    walkProof: null,
    landed: null,
    error: null,
    updatedMs: 0,
  };
}

export const DEFECTS: RigRecord['defects'] = [
  {
    number: 1,
    title: 'Cape weighted to the arm bones',
    place: 'cape, both sides',
    tags: ['LeftArm 2,104 v', 'RightArm 1,988 v', '3,912 v in all'],
    kind: 'weights-only',
    detail:
      'In raise and forward the cape swings out with the arms. Fix: reweight the cape island to the trunk with a 5 cm seam at the sleeve. A vertex can count under several bones, so the bones add up to more than the total.',
    outcome: null,
    fixed: false,
  },
  {
    number: 2,
    title: 'Hair strands on finger bones',
    place: 'hair, back',
    tags: ['LeftHandIndex1 652 v', 'RightHandIndex1 632 v', '1,284 v in all'],
    kind: 'weights-only',
    detail:
      'In hand two strands follow the fingers. Fix: hand them to the head and neck. A vertex can count under several bones, so the bones add up to more than the total.',
    outcome: null,
    fixed: false,
  },
  {
    number: 3,
    title: 'Left arm chain off its mesh',
    place: 'left arm',
    tags: ['10 cm off', '23 joints'],
    kind: 'needs-yes',
    detail:
      'Weights get mirrored from the right arm. Moving the 23 joints onto the mesh changes the rest pose, so it waits for your yes.',
    outcome: null,
    fixed: false,
  },
];

/** The first board: the pose test ran, the fix is next. */
export function testedCase(id: string, name: string): RigRecord {
  return {
    ...readCase(id, name, null),
    state: 'tested',
    step: 'before',
    before: sheet(`${id}/before`, HUMAN_ROWS, false),
    defects: DEFECTS,
    stats: [
      { label: 'Hand weight far from the hand', before: null, after: '1,284', bad: true },
      { label: 'Cloth on arm bones', before: null, after: '3,912', bad: true },
      { label: 'Worst stretch, hand, every view', before: null, after: '244 mm', bad: false },
      { label: 'Past the body, hand, every view', before: null, after: '236 mm', bad: true },
      { label: 'Seam gaps, cast_orb 0.5, every view', before: null, after: '393 mm', bad: true },
    ],
  };
}

/** The second board: the draft and its checks, with the joint move asked. */
export function fixedCase(id: string, name: string): RigRecord {
  const tested = testedCase(id, name);
  const plan = HUMAN_ROWS;
  return {
    ...tested,
    state: 'fixed',
    step: 'land',
    round: 1,
    draft: `${DIR}/${id}/draft.glb`,
    after: sheet(`${id}/after`, plan, true),
    defects: DEFECTS.map(d => ({
      ...d,
      fixed: d.kind !== 'needs-yes',
      outcome: d.kind === 'needs-yes' ? 'Weights mirrored, joints wait for a yes' : 'fixed',
    })),
    stats: [
      { label: 'Hand weight far from the hand', before: '1,284', after: '0', bad: false },
      { label: 'Cloth on arm bones', before: '3,912', after: '146', bad: false },
      {
        label: 'Worst stretch, death 0.5, every view',
        before: '244 mm',
        after: '275 mm',
        bad: true,
      },
      {
        label: 'Past the body, death 0.5, every view',
        before: '236 mm',
        after: '57 mm',
        bad: false,
      },
      { label: 'Seam gaps', before: '393 mm', after: '0 mm', bad: false },
      { label: 'Seam band', before: null, after: '5 cm', bad: false },
      { label: 'New weights', before: null, after: '5,330 v', bad: false },
    ],
    fixSteps: [
      {
        title: 'Reweight each island',
        detail: 'Cape and hair go to the trunk and head. Cuffs and gloves stay on the arm.',
        value: '5,330 v',
      },
      {
        title: 'Mirror the left arm',
        detail: 'Weights copied from the right, since its joints sit off the mesh.',
        value: '1,104 v',
      },
      {
        title: 'Smooth the seams',
        detail: 'A geodesic band where cape meets sleeve, so it stretches, not tears.',
        value: '5 cm',
      },
    ],
    checks: [
      {
        ok: true,
        title: 'Only JOINTS_0 and WEIGHTS_0 changed',
        detail: 'Mesh, UVs and the texture are byte for byte the same',
        value: '2 of 14',
      },
      {
        ok: true,
        title: 'Bones and rest pose unchanged',
        detail: 'Names, parents and heads match the source',
        value: '65',
      },
      {
        ok: true,
        title: 'At most 4 influences, sums of 1',
        detail: '0 vertices without weight',
        value: 'max 4',
      },
      {
        ok: true,
        title: 'Clips read back',
        detail: 'idle and walk import with every bone',
        value: '2',
      },
    ],
    jointMove: {
      state: 'asked',
      limb: 'left arm',
      offCm: 10,
      joints: 23,
      breaksClips: ['walk'],
      draft: `${DIR}/${id}/draft_moved.glb`,
      sheet: sheet(`${id}/moved`, plan, true, true),
    },
    stages: [
      { name: 'Fix', detail: '5,330 vertices changed', seconds: 6 },
      { name: 'Verify', detail: 'Only joints and weights', seconds: 2 },
      { name: 'Pose test after', detail: '9 rows', seconds: 13 },
    ],
  };
}
