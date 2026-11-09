import { useCallback, useEffect, useEffectEvent, useMemo, useRef, useState } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import type { RecipeLayer } from '@/ipc/client';
import { LOOK_BONES, limbOf, mixamoName } from '@/state/recipe-bones';
import type { PinCheck } from '@/state/recipe-pins';
import type { ReachCheck } from '@/state/recipe-review';
import { checkReaches } from '../check-reaches';
import {
  checkPins,
  holdPins,
  pinSpots,
  poseAtOf,
  type PinSpot,
  type StagePins,
} from '../recipe-pins';
import {
  poseRecipe,
  rigForward,
  targetsOf,
  type RecipeMarker,
  type RecipeRig,
} from '../recipe-pose';
import { joinComposer } from '../recipe-join';
import { roughRetarget, type RoughPair } from '../rough-retarget';
import { frameOfShare, type Playable, type Timeline } from '@/state/recipe-timeline';
import type { LoadedMotion } from '../MotionStage/Scene/Scene.types';

/** The model lined up to follow the clip (or in its rest pose), and what poses it at a share of the clip. */
export interface RecipeSetup {
  rig: RecipeRig;
  /** Puts the clip's pose at `share` back on the model, with no change laid over. */
  frameAt: (share: number) => void;
  /**
   * Joined clips only: poses the bake at `playhead` (a share of its
   * frames), with no change laid over, and answers where the changes stand there.
   */
  playAt?: (playhead: number) => number;
  /** Stops the clip's mixer and puts the model and skeleton back as they came. */
  stop: () => void;
}

/** A recipe of joined clips: each clip's skeleton, its rate and the timeline they play. */
export interface JoinedPlay {
  skeletons: readonly LoadedMotion[];
  fps: readonly number[];
  timeline: Timeline;
}

const rotationOf = (o: THREE.Object3D) => o.getWorldQuaternion(new THREE.Quaternion());

type Snapshot = (readonly [THREE.Object3D, THREE.Vector3, THREE.Quaternion])[];

/** Each loaded scene's bones as they came, taken the first time it is set up, before any pose. */
const asLoaded = new WeakMap<THREE.Object3D, Snapshot>();

/** The bones under `root` as they stand; the scene itself is left to the framing. */
function take(root: THREE.Object3D): Snapshot {
  const out: Snapshot = [];
  root.traverse(o => {
    if (o instanceof THREE.Bone) out.push([o, o.position.clone(), o.quaternion.clone()] as const);
  });
  return out;
}

function put(snapshot: Snapshot) {
  for (const [o, position, quaternion] of snapshot) {
    o.position.copy(position);
    o.quaternion.copy(quaternion);
  }
}

/** Puts `root` back as it came, remembering that the first time it is seen. */
function unposed(root: THREE.Object3D) {
  const known = asLoaded.get(root);
  if (known) put(known);
  else asLoaded.set(root, take(root));
  root.updateWorldMatrix(true, true);
}

/** No clip to follow: the model holds its rest pose, as the bake does. */
const REST_POSE = { frameAt: () => undefined, timeAt: () => undefined, stop: () => undefined };

/** The base clip of `skeleton` played on `model` through the bone pairs, or null. */
function following(model: LoadedMotion, skeleton: LoadedMotion, pairs: readonly RoughPair[]) {
  const clip = skeleton.clips[0];
  if (!clip) return null;
  unposed(skeleton.scene);
  const pose = roughRetarget(skeleton.scene, model.scene, clip, pairs);
  if (!pose) return null;
  return {
    frameAt: (share: number) => pose(share * clip.duration),
    timeAt: (time: number) => pose(Math.min(Math.max(time, 0), clip.duration)),
    stop: () => {
      pose.stop();
      unposed(skeleton.scene);
    },
  };
}

function setUp(
  model: LoadedMotion,
  skeleton: LoadedMotion | 'rest',
  pairs: readonly RoughPair[],
  joined: JoinedPlay | null = null
): RecipeSetup | null {
  // Each joined clip's own retarget, made before the base's so the base's lined up rest stays.
  const others = new Map<LoadedMotion, ReturnType<typeof following>>();
  for (const other of joined?.skeletons ?? []) {
    if (other === skeleton || others.has(other)) continue;
    unposed(model.scene);
    others.set(other, following(model, other, pairs));
  }
  // A model or clip set up before (another clip, the same model) starts again as it came.
  unposed(model.scene);
  const bone = (vrm: string) => {
    const name = mixamoName(vrm);
    const target = pairs.find(p => p.bone === name)?.target;
    return target
      ? model.scene.getObjectByName(THREE.PropertyBinding.sanitizeNodeName(target))
      : undefined;
  };
  // Read from the rig as it came, before the retarget lines its limbs up.
  const forward = rigForward(bone);
  const restWorld = new Map<THREE.Object3D, THREE.Quaternion>();
  model.scene.traverse(o => {
    if (o instanceof THREE.Bone) restWorld.set(o, rotationOf(o));
  });
  const base = skeleton === 'rest' ? REST_POSE : following(model, skeleton, pairs);
  if (!base) return null;
  // Every frame starts from the lined up rest, so a nudge's move never adds up.
  const rest = take(model.scene);
  const facings = new Map<THREE.Object3D, THREE.Vector3>();
  for (const vrm of LOOK_BONES) {
    const b = bone(vrm);
    if (b) facings.set(b, forward.clone().applyQuaternion(rotationOf(b).invert()));
  }
  const rig: RecipeRig = {
    bone,
    limb: end => {
      const names = limbOf(end);
      const upper = names ? bone(names.upper) : undefined;
      const lower = names ? bone(names.lower) : undefined;
      return upper && lower ? { upper, lower } : undefined;
    },
    forward,
    facing: b => facings.get(b) ?? new THREE.Vector3(0, 0, 1),
    rest: b => restWorld.get(b),
  };
  const atRest = () => {
    put(rest);
    model.scene.updateWorldMatrix(true, true);
  };
  const stop = () => {
    base.stop();
    for (const other of others.values()) other?.stop();
    unposed(model.scene);
  };
  if (!joined) {
    return {
      rig,
      frameAt: share => {
        atRest();
        base.frameAt(share);
      },
      stop,
    };
  }
  const posers = joined.skeletons.map(k => (k === skeleton ? base : others.get(k)));
  const bones = pairs.flatMap(p => {
    const b = p.target ? named(model.scene, p.target) : undefined;
    return b ? [b] : [];
  });
  const byName = new Map(pairs.flatMap(p => (p.target ? [[p.bone, p.target] as const] : [])));
  const { poseFrame } = joinComposer(
    {
      bones,
      hips: bone('hips') ?? null,
      partner: b => {
        const pair = pairs.find(p => p.target && named(model.scene, p.target) === b);
        const other = pair ? byName.get(otherSideOf(pair.bone)) : undefined;
        return (other ? named(model.scene, other) : undefined) ?? b;
      },
      chest: bone('chest') ?? null,
      spine: bone('spine') ?? null,
      forward,
      origin: model.scene.position.clone(),
      rest: atRest,
      put: (clip, at) => posers[clip]?.timeAt(at / (joined.fps[clip] || 30)),
    },
    joined.timeline
  );
  return {
    rig,
    frameAt: share => {
      poseFrame(frameOfShare(joined.timeline, share));
    },
    playAt: playhead => poseFrame(playhead * joined.timeline.frames),
    stop,
  };
}

const named = (root: THREE.Object3D, name: string) =>
  root.getObjectByName(THREE.PropertyBinding.sanitizeNodeName(name));

/** A humanoid (Mixamo) bone name's partner on the other side: LeftArm and RightArm. */
const otherSideOf = (name: string) =>
  name.startsWith('Left')
    ? `Right${name.slice(4)}`
    : name.startsWith('Right')
      ? `Left${name.slice(5)}`
      : name;

/**
 * Lines `model` up with the clip of `skeleton` through the bone pairs, or
 * holds its rest pose for `'rest'`, afresh for each new one.
 */
export function useRecipeSetup(
  model: LoadedMotion | null,
  skeleton: LoadedMotion | 'rest' | null,
  pairs: readonly RoughPair[],
  joined: JoinedPlay | null = null
): RecipeSetup | null {
  const [setup, setSetup] = useState<RecipeSetup | null>(null);
  useEffect(() => {
    const next = model && skeleton ? setUp(model, skeleton, pairs, joined) : null;
    setSetup(next);
    return () => next?.stop();
  }, [model, skeleton, pairs, joined]);
  return setup;
}

/**
 * Where each blank's target sits, from the model's height once it is framed
 * and the way its rig faces (+Z until it is set up). Targets are in the
 * model's own frame, as the bake places them; framing moves the model to
 * stand on the floor, so they move with it (`origin`, where its own origin
 * ended up), or a model whose bounds do not start at its feet would reach
 * for a spot the bake never aims at.
 */
export function useTargets(
  markers: readonly RecipeMarker[],
  box: THREE.Box3 | null,
  forward: THREE.Vector3 | null,
  origin: THREE.Vector3 | null = null
) {
  const key = JSON.stringify(markers);
  const height = box ? box.max.y - box.min.y : 0;
  const [x, z] = forward ? [forward.x, forward.z] : [0, 1];
  const [ox, oy, oz] = origin ? [origin.x, origin.y, origin.z] : [0, 0, 0];
  return useMemo(() => {
    const targets = targetsOf(
      JSON.parse(key) as RecipeMarker[],
      height || 1.7,
      new THREE.Vector3(x, 0, z)
    );
    const shift = new THREE.Vector3(ox, oy, oz);
    for (const at of targets.values()) at.add(shift);
    return targets;
  }, [key, height, x, z, ox, oy, oz]);
}

/** Poses before the frame's other work, so the pose keys gizmo reads the pose laid. */
const POSE_FIRST = -1;

/**
 * Poses the clip at `share` each frame with the changes laid over it and
 * the pins held last, moving the playhead while it plays, and measures each
 * reach's end, takes each pin's spot and checks it again whenever the
 * changes or targets change.
 */
export function useRecipeFrame(
  setup: RecipeSetup | null,
  layers: readonly RecipeLayer[],
  targets: ReadonlyMap<string, THREE.Vector3>,
  play: { share: number; playing: boolean; onTick: (delta: number) => void },
  onChecks?: (checks: ReachCheck[]) => void,
  pins: StagePins | null = null,
  keysOn: Playable | null = null
) {
  const report = useEffectEvent((checks: ReachCheck[]) => onChecks?.(checks));
  const spots = useRef<ReadonlyMap<string, PinSpot>>(new Map());
  const reportPins = useEffectEvent((checks: PinCheck[]) => pins?.onChecks?.(checks));
  useEffect(() => {
    if (!setup) return;
    report(checkReaches(setup.frameAt, setup.rig, layers, targets));
    const spans = pins?.spans ?? [];
    const poseAt = poseAtOf(setup, pins?.frames ?? 0);
    spots.current = pinSpots(poseAt, setup.rig, layers, targets, spans, keysOn);
    reportPins(checkPins(poseAt, setup.rig, layers, targets, spans, spots.current, keysOn));
  }, [setup, layers, targets, pins, keysOn]);
  useFrame((_, delta) => {
    if (!setup) return;
    if (play.playing) play.onTick(delta);
    let share = play.share;
    if (setup.playAt) share = setup.playAt(play.share);
    else setup.frameAt(play.share);
    poseRecipe(setup.rig, layers, targets, share, keysOn);
    if (pins?.spans.length)
      holdPins(setup.rig, pins.spans, spots.current, play.share * pins.frames);
  }, POSE_FIRST);
}

/**
 * The other clips' skeletons of a joined recipe: which files still load
 * (`extra`, each once), what to call as each comes in, and once every one
 * is in, what the setup plays.
 */
export function useJoinedSkeletons(
  clipUrl: string | null,
  skeleton: LoadedMotion | null,
  joined: { urls: readonly string[]; fps: readonly number[]; timeline: Timeline } | null
) {
  const [loaded, setLoaded] = useState<Readonly<Record<string, LoadedMotion>>>({});
  const urls = joined?.urls;
  const extra = useMemo(() => [...new Set(urls ?? [])].filter(u => u !== clipUrl), [urls, clipUrl]);
  const onLoad = useCallback((url: string, next: LoadedMotion | null) => {
    setLoaded(was => {
      const out = { ...was };
      if (next) out[url] = next;
      else delete out[url];
      return out;
    });
  }, []);
  const play = useMemo<JoinedPlay | null>(() => {
    if (!joined || !skeleton) return null;
    const skeletons = joined.urls.map(u => (u === clipUrl ? skeleton : loaded[u]));
    if (!skeletons.every(Boolean)) return null;
    return { skeletons: skeletons as LoadedMotion[], fps: joined.fps, timeline: joined.timeline };
  }, [joined, skeleton, loaded, clipUrl]);
  return { extra, onLoad, play };
}
