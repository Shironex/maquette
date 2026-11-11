/**
 * The mock grip library (`?library=demo`): six grips, as a small weapon
 * rack would have them. Longsword, two hands has three revisions and the
 * scythe's staff grip two, so a recipe that copied an older one is offered
 * the newest; the dagger, the rapier and the shield strap pose one hand and
 * the relaxed empty hand goes on either. Plus the recipe that uses them:
 * Scythe charge, held keys both hands in a pose key change (the hand pose a
 * grip can be saved from) and holds the scythe's staff grip as copied from
 * r1, while the library has r2.
 */
import type {
  AnimationClip,
  GripFile,
  GripHolds,
  LibraryGripEntry,
  LibraryGripRevision,
  RecipeFile,
} from '../client';
import { clock } from '@/lib/clock';
import { GRIP_VERSION, layerOf, mirrorGrip, sidePose, type GripPose } from '@/state/recipe-grips';
import { baseOf, layer, newRecipe } from './recipe-files';

const DAY = 86_400_000;
const ago = (days: number) => new Date(clock.nowMs() - days * DAY).toISOString();

/** A library grip and its revisions' files, by revision. */
export interface StoredGrip {
  entry: LibraryGripEntry;
  files: Map<number, GripFile>;
}

/** The demo recipe that keys the hands and holds a grip. */
export const GRIP_DEMO = 'scythe-charge-held';
/** The scythe grip's uid, which the demo recipe's grip change links to. */
export const SCYTHE_UID = '5eed0000000000b2';

const FINGERS = ['Index', 'Middle', 'Ring', 'Little'] as const;
const JOINTS = ['Proximal', 'Intermediate', 'Distal'] as const;

/**
 * A right hand closed around something: the forearm turned by `forearm`
 * degrees, the wrist by `wrist`, each finger joint curled toward the palm
 * (the little finger a little more than the index) and the thumb folded
 * across. `forearm` null leaves the forearm out.
 */
function rightHand(
  curl: readonly [number, number, number],
  wrist: readonly [number, number, number],
  forearm: number | null
): GripPose {
  const pose: GripPose = { rightHand: [...wrist] };
  if (forearm !== null) pose.rightLowerArm = [0, forearm, 0];
  FINGERS.forEach((finger, f) =>
    JOINTS.forEach((joint, j) => {
      pose[`right${finger}${joint}`] = [0, 0, curl[j]! + f * 3];
    })
  );
  pose.rightThumbMetacarpal = [18, -24, 10];
  pose.rightThumbProximal = [0, -16, Math.round(curl[0] / 3)];
  pose.rightThumbDistal = [0, -12, Math.round(curl[1] / 4)];
  return pose;
}

/** Both hands: the right as given, the left its mirror, with no left forearm. */
function bothHands(
  curl: readonly [number, number, number],
  wrist: readonly [number, number, number],
  forearm: number
): GripPose {
  const right = rightHand(curl, wrist, forearm);
  const left = mirrorGrip(rightHand(curl, wrist, null));
  return { ...left, ...right };
}

const grip = (
  name: string,
  revision: number,
  weapon: string,
  hands: GripHolds,
  pose: GripPose
): GripFile => ({ version: GRIP_VERSION, name, revision, weapon, hands, pose });

const BOTH: GripHolds = { holds: 'right', supports: 'left' };
const RIGHT: GripHolds = { holds: 'right', supports: null };
const LEFT: GripHolds = { holds: 'left', supports: null };

/** A revision line: saved from a pose key change of a project's recipe. */
export function gripRevision(
  revision: number,
  savedAt: string,
  from: { project: string; recipe: string; layer: string; frame: number } | null = null
): LibraryGripRevision {
  return {
    revision,
    file: `r${revision}.grip.json`,
    savedAt,
    project: from?.project ?? null,
    recipe: from?.recipe ?? null,
    layer: from?.layer ?? null,
    frame: from?.frame ?? null,
  };
}

let made = 0;
/** A fresh uid for a mock library grip: 16 hex digits. */
export const newGripUid = () =>
  (clock.nowMs() * 1000 + 500 + made++).toString(16).padStart(16, '0').slice(-16);

/** An entry whose revisions are `files`, oldest first, saved `days` apart. */
export function storedGrip(id: string, files: readonly GripFile[], uid = newGripUid()): StoredGrip {
  const newest = files[files.length - 1]!;
  const from = { project: 'frostveil', recipe: GRIP_DEMO, layer: 'l-1', frame: 30 };
  const revisions = files.map((file, i) =>
    gripRevision(file.revision, ago((files.length - i) * 3), from)
  );
  return {
    entry: {
      version: 1,
      id,
      uid,
      name: newest.name,
      createdAt: revisions[0]!.savedAt,
      updatedAt: revisions[revisions.length - 1]!.savedAt,
      lastRevision: newest.revision,
      revisions,
    },
    files: new Map(files.map(file => [file.revision, file])),
  };
}

/** The scythe's staff grip, r1 and r2: r2 closes the fingers further. */
const scythe = (revision: number) =>
  grip(
    'Scythe, staff grip',
    revision,
    'scythe',
    BOTH,
    bothHands(revision === 1 ? [50, 56, 38] : [62, 70, 44], [6, 0, -10], 14)
  );

export function demoGrips(): Map<string, StoredGrip> {
  const longsword = (revision: number) =>
    grip(
      'Longsword, two hands',
      revision,
      'longsword',
      BOTH,
      bothHands([58 + revision * 2, 72, 48], [4, 0, -8], 10)
    );
  const relaxed = (revision: number) =>
    grip(
      'Empty hand, relaxed',
      revision,
      'empty',
      RIGHT,
      rightHand([12 + revision, 18, 10], [0, 0, 0], null)
    );
  const shield = sidePose(mirrorGrip(rightHand([66, 60, 30], [0, 12, 0], 20)), 'left');
  const entries = [
    storedGrip('longsword-two-hands', [1, 2, 3].map(longsword)),
    storedGrip('scythe-staff-grip', [1, 2].map(scythe), SCYTHE_UID),
    storedGrip('dagger-reverse', [
      grip('Dagger, reverse', 1, 'dagger', RIGHT, rightHand([70, 78, 52], [0, 30, 0], -18)),
    ]),
    storedGrip('rapier-sabre-grip', [
      grip('Rapier, sabre grip', 1, 'rapier', RIGHT, rightHand([44, 60, 36], [8, 0, 0], 6)),
    ]),
    storedGrip('shield-strap', [grip('Shield, strap', 1, 'shield', LEFT, shield)]),
    storedGrip('empty-hand-relaxed', [1, 2, 3, 4].map(relaxed)),
  ];
  return new Map(entries.map(e => [e.entry.id, e]));
}

/**
 * Frostveil's demo of grips: a sword idle with both hands keyed in a pose
 * key change (the right hand and forearm at frame 20, the left hand at 30,
 * so frame 30 holds 33 bones) and the scythe's staff grip laid over most
 * of it, as copied from r1.
 */
export function gripRecipe(clips: readonly AnimationClip[]): RecipeFile | null {
  const idle = clips.find(c => c.name === 'Sword And Shield Idle');
  if (!idle) return null;
  const recipe = newRecipe('Scythe charge, held', baseOf(idle));
  recipe.version = 2;
  // Closed a little further than r2, the charge's tighter hold: a revision worth saving.
  const keyed = bothHands([66, 74, 48], [6, 0, -10], 14);
  const held = layerOf(
    'l-2',
    scythe(1),
    { entry: 'scythe-staff-grip', uid: SCYTHE_UID, revision: 1 },
    'both'
  );
  recipe.layers = [
    {
      ...layer('l-1', 'pose', '', {}),
      mode: 'replace',
      keys: [
        { clip: 0, frame: 20, ease: 'smooth', pose: sidePose(keyed, 'right') },
        { clip: 0, frame: 30, ease: 'smooth', pose: sidePose(keyed, 'left') },
      ],
    },
    { ...held, settings: { ...held.settings, start: 0.1, end: 0.9 } },
  ];
  return recipe;
}
