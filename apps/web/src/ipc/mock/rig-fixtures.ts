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
export function figure(pose: string, view: string, wrong: boolean, moved = false): string {
  const lift = LIFT[pose] ?? 22;
  const armY = 44 + lift;
  const legs =
    pose === 'stride' ? 'M50 72 L38 104 M50 72 L62 104' : 'M46 72 L44 104 M54 72 L56 104';
  const cape = wrong
    ? `<path d="M40 36 L${18} ${armY + 6} L38 78 Z" fill="#d67f5f" opacity=".85"/>`
    : '<path d="M42 36 L36 80 L64 80 L58 36 Z" fill="#7d8aa0" opacity=".7"/>';
  const left = moved ? 26 : 30;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 110"><rect width="100" height="110" fill="#c7c9cf"/><text x="4" y="10" font-size="6" fill="#55585f" font-family="monospace">${pose} ${view}</text>${cape}<circle cx="50" cy="24" r="8" fill="#3a3d44"/><path d="M50 32 L50 72" stroke="#3a3d44" stroke-width="9" stroke-linecap="round"/><path d="M50 38 L${left} ${armY} M50 38 L72 ${armY}" stroke="#3a3d44" stroke-width="5" stroke-linecap="round"/><path d="${legs}" stroke="#3a3d44" stroke-width="6" stroke-linecap="round"/></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function picture(path: string, pose: string, view: string, wrong: boolean, moved = false): string {
  registerMockFile(path, figure(pose, view, wrong, moved));
  return path;
}

export interface RowPlan {
  id: string;
  note?: string;
  defects?: number[];
  stretch?: number;
}

export const HUMAN_ROWS: RowPlan[] = [
  { id: 'rest', stretch: 4 },
  { id: 'raise', note: 'cape swings', defects: [1], stretch: 62 },
  { id: 'forward', note: 'cape swings', defects: [1], stretch: 48 },
  { id: 'hand', note: 'hair follows', defects: [2], stretch: 31 },
  { id: 'twist', stretch: 9 },
  { id: 'head', stretch: 6 },
  { id: 'stride', stretch: 11 },
  { id: 'clip:walk:0.5', note: 'clip frame', stretch: 12 },
  { id: 'left arm', note: 'joints off mesh', defects: [3], stretch: 20 },
];

function rows(tag: string, plan: RowPlan[], fixed: boolean, moved = false): RigRow[] {
  return plan.map(p => {
    const wrong = !fixed && Boolean(p.defects?.length);
    const cell = (view: string) =>
      picture(`${DIR}/${tag}/${p.id}_${view}.svg`, p.id, view, wrong, moved);
    return {
      id: p.id,
      label: p.id.replace('clip:', '').replace(':', ' '),
      note: fixed ? (p.defects?.length ? 'holds now' : null) : (p.note ?? null),
      defects: fixed ? [] : (p.defects ?? []),
      stretchMm: fixed ? 12 : (p.stretch ?? 5),
      seamGapMm: 0,
      front: cell('front'),
      side: cell('side'),
      back: cell('back'),
      xray: cell('xray'),
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
      rig: 'Mixamo',
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
    tags: ['LeftArm', 'RightArm', '3,912 v'],
    kind: 'weights-only',
    detail:
      'In raise and forward the cape swings out with the arms. Fix: reweight the cape island to the trunk with a 5 cm seam at the sleeve.',
    outcome: null,
    fixed: false,
  },
  {
    number: 2,
    title: 'Hair strands on finger bones',
    place: 'hair, back',
    tags: ['LeftHandIndex1 652 v', 'RightHandIndex1 632 v'],
    kind: 'weights-only',
    detail: 'In hand two strands follow the fingers. Fix: hand them to the head and neck.',
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
      { label: 'Worst stretch, raise', before: null, after: '62 mm', bad: true },
      { label: 'Seam gaps', before: null, after: '0 mm', bad: false },
    ],
  };
}

/** The second board: the draft and its checks, with the joint move asked. */
export function fixedCase(id: string, name: string): RigRecord {
  const tested = testedCase(id, name);
  const plan = HUMAN_ROWS.slice(0, 4);
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
      { label: 'Hair on finger bones', before: '1,284', after: '0', bad: false },
      { label: 'Cloth on arm bones', before: '3,912', after: '146', bad: false },
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
      { name: 'Pose test after', detail: '4 rows', seconds: 13 },
    ],
  };
}
