/** The rig doctor cases for the other boards and stops, built on rig-fixtures.ts. */
import type { RigRecord } from '../client';
import { registerMockFile } from '../files';
import {
  DEFECTS,
  DIR,
  HUMAN_ROWS,
  figure,
  fixedCase,
  readCase,
  sheet,
  testedCase,
  type RowPlan,
} from './rig-fixtures';

/** The third board: an agent's case on a creature, running. */
export function creatureCase(): RigRecord {
  const id = 'spiked-hound-v1';
  const plan: RowPlan[] = [
    { id: 'leg_RR', note: 'tears 91 mm', defects: [1], stretch: 91 },
    { id: 'tail', note: '12 pieces bend 18 mm', defects: [2], stretch: 22 },
    { id: 'leg_FR', stretch: 8 },
    { id: 'neck', stretch: 6 },
  ];
  const base = readCase(id, 'Spiked hound', null);
  const map = `${DIR}/${id}/map_persp.svg`;
  registerMockFile(map, figure('weights', 'persp', true));
  return {
    ...base,
    route: 'creature',
    startedBy: 'agent',
    state: 'testing',
    step: 'verify',
    source: { inbox: null, project: 'frostveil', asset: 'spiked-hound', version: 1 },
    model: {
      ...base.model,
      rig: 'Own names',
      joints: 87,
      clips: [],
      finding: 'Tripo bone names, so it takes the creature route.',
    },
    creature: {
      front: '-X',
      legs: 4,
      swapped: 2,
      junk: ['dup_FL', 'dup_FR'],
      rigidPieces: 41,
      oneShell: false,
      chains: [
        { chain: 'leg_FL', bones: 4, side: 'left', readAs: 'chain', note: null },
        {
          chain: 'leg_FR',
          bones: 4,
          side: 'right',
          readAs: 'swapped',
          note: 'Named Left, sits on the right',
        },
        {
          chain: 'leg_RR',
          bones: 4,
          side: 'right',
          readAs: 'stray',
          note: 'Pulls the tail, 200 v',
        },
        { chain: 'tail', bones: 6, side: 'middle', readAs: 'chain', note: null },
        { chain: 'dup_FL', bones: 3, side: 'left', readAs: 'junk', note: 'On top of leg_FL' },
      ],
      weightMap: map,
    },
    before: sheet(`${id}/before`, plan, false),
    after: sheet(`${id}/after`, plan, true),
    defects: [
      {
        number: 1,
        title: 'The leg_RR pulls tail',
        place: 'leg_RR',
        tags: ['200 v'],
        kind: 'weights-only',
        detail:
          'Vertices of the leg_RR carry weight of tail, far from it. Fix: take that weight off and fill in from neighbours.',
        outcome: null,
        fixed: false,
      },
      {
        number: 2,
        title: 'Rigid pieces split between bones',
        place: 'spikes and plates',
        tags: ['41 pieces', 'one bone each'],
        kind: 'weights-only',
        detail: 'They bend when a chain moves. Fix: each small piece follows exactly one bone.',
        outcome: null,
        fixed: false,
      },
    ],
    progress: { label: 'Pose test after', done: 9, total: 13 },
    stages: [
      { name: 'Chains and weight map', detail: '87 bones, 13 chains', seconds: 11 },
      { name: 'Pose test before', detail: '13 chains', seconds: 74 },
      { name: 'Fix, dry run checked', detail: '3,104 vertices changed', seconds: 9 },
      { name: 'Verify', detail: 'Rest pose and bones checked', seconds: 3 },
      { name: 'Pose test after', detail: '', seconds: null },
    ],
    log: [
      {
        at: '14:02:11',
        script: 'fix.py',
        text: '3,104 vertices changed, joints and weights patched',
      },
      { at: '14:02:14', script: 'verify.py', text: '2 accessors changed, max 4' },
      { at: '14:03:02', script: 'posetest.py', text: 'bending leg_RR' },
    ],
  };
}

/** The fourth board's first card. */
export function refusedCase(): RigRecord {
  const base = readCase('crystal-golem-inbox', 'Crystal golem', 'crystal-golem.glb');
  return {
    ...base,
    route: 'creature',
    state: 'refused',
    step: 'read',
    model: { ...base.model, skinnedMeshes: 3, finding: null },
    issues: [
      {
        ok: false,
        title: 'Three skinned meshes',
        detail:
          'The scripts patch one skinned primitive, so a byte level fix cannot reach all of them.',
      },
      { ok: true, title: 'Bones read', detail: '65 joints with Mixamo names.' },
    ],
  };
}

/** The fourth board's second card. */
export function stoppedCase(): RigRecord {
  const fixed = fixedCase('marrow-witch-v1', 'Marrow witch');
  return {
    ...fixed,
    state: 'stopped',
    round: 2,
    jointMove: null,
    stop: {
      kind: 'rounds',
      title: 'Stopped after two rounds: Marrow witch',
      detail:
        'raise still stretches 74 mm on the after sheet. The doctor stops here instead of guessing a third time.',
      row: 'raise',
    },
  };
}

/** One continuous mesh: the narrow reweight is what the fix means here. */
export function oneIslandCase(): RigRecord {
  const tested = testedCase('rowan-shell-v1', 'Rowan shell');
  const plan = HUMAN_ROWS.map(r =>
    r.id === 'hand' || r.id === 'left arm' ? { id: r.id, stretch: 9 } : r
  );
  return {
    ...tested,
    before: sheet('rowan-shell-v1/before', plan, false),
    model: { ...tested.model, islands: 1, oneIsland: true, finding: null },
    defects: [
      {
        ...DEFECTS[0]!,
        detail:
          'In raise and forward the cloth swings with the arms. Fix: move that weight to the trunk with a faded seam.',
      },
      {
        number: 2,
        title: 'One continuous mesh',
        place: 'the whole body',
        tags: ['1 island'],
        kind: 'note',
        detail:
          'The per island fix would tear sleeves and legs here, so it does not run. A narrow reweight moves only the arm and hand weight far from the arms.',
        outcome: null,
        fixed: false,
      },
    ],
  };
}

/** A pose test partway through. */
export function runningCase(): RigRecord {
  return {
    ...testedCase('pale-knight-v1', 'Pale knight'),
    state: 'testing',
    progress: { label: 'Pose test before', done: 9, total: 27 },
    defects: [],
    stats: [],
  };
}
