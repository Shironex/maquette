/**
 * Posing a VRM through its normalised humanoid bones, which every VRM
 * shares whatever its rig: each starts at rest with no rotation, the
 * character faces +Z with its left on +X, and the arms lie along X in a
 * T-pose. Arms down turns the upper arms down about Z and bends the elbows
 * forward a little; breathe lifts the chest and the arms on a slow cycle;
 * a wiggle shakes the hips and the head for a moment, so spring bones
 * swing.
 * Plain three.js, so it is tested without three-vrm.
 */
import * as THREE from 'three';

/** The 15 bones VRM 1.0 requires. */
export const REQUIRED_BONES = [
  'hips',
  'spine',
  'head',
  'leftUpperLeg',
  'leftLowerLeg',
  'leftFoot',
  'rightUpperLeg',
  'rightLowerLeg',
  'rightFoot',
  'leftUpperArm',
  'leftLowerArm',
  'leftHand',
  'rightUpperArm',
  'rightLowerArm',
  'rightHand',
] as const;

/** What the pose needs of a VRM humanoid. */
export interface PosableHumanoid {
  getNormalizedBoneNode: (name: never) => THREE.Object3D | null;
}

export interface PoseOptions {
  /** Arms down from the T-pose; else the T-pose as written. */
  armsDown: boolean;
  /** A slow breath in the chest and arms. */
  breathe: boolean;
  /** Seconds since a wiggle started; null without one. */
  wiggle?: number | null;
}

/** How long a wiggle lasts, and how far it turns the hips and tilts the head at first. */
export const WIGGLE_SECONDS = 1.6;
const WIGGLE_HIPS = THREE.MathUtils.degToRad(24);
const WIGGLE_HEAD = THREE.MathUtils.degToRad(16);

/** A wiggle's sway `seconds` in, from -1 to 1, dying down to 0 at its end. */
export function wiggleAt(seconds: number | null | undefined): number {
  if (seconds === null || seconds === undefined || seconds < 0 || seconds >= WIGGLE_SECONDS) {
    return 0;
  }
  const fade = 1 - seconds / WIGGLE_SECONDS;
  return Math.sin(seconds * Math.PI * 2 * 2.5) * fade * fade;
}

/** How far the arms come down from level. */
const ARMS_DOWN = THREE.MathUtils.degToRad(68);
/** How far the elbows bend forward with the arms down. */
const ELBOWS = THREE.MathUtils.degToRad(14);
/** Seconds per breath. */
const BREATH_SECONDS = 4.2;
/** How far the chest tips back, and the arms lift, at the top of a breath. */
const CHEST_BREATH = THREE.MathUtils.degToRad(1.6);
const ARM_BREATH = THREE.MathUtils.degToRad(2.2);

function bone(humanoid: PosableHumanoid, name: string): THREE.Object3D | null {
  return humanoid.getNormalizedBoneNode(name as never);
}

/** The breath at `seconds`, from 0 (out) to 1 (in). */
export function breathAt(seconds: number): number {
  return (1 - Math.cos((seconds / BREATH_SECONDS) * Math.PI * 2)) / 2;
}

/**
 * Sets the arms, elbows and chest of `humanoid` for `seconds` into the idle.
 * Every bone it touches is set each time, so turning an option off puts
 * the bone back to rest.
 */
export function poseHumanoid(
  humanoid: PosableHumanoid,
  seconds: number,
  { armsDown, breathe, wiggle }: PoseOptions
): void {
  const breath = breathe ? breathAt(seconds) : 0;
  const lift = (armsDown ? ARMS_DOWN : 0) - breath * ARM_BREATH;
  const elbows = armsDown ? ELBOWS : 0;
  for (const [side, sign] of [
    ['left', -1],
    ['right', 1],
  ] as const) {
    bone(humanoid, `${side}UpperArm`)?.rotation.set(0, 0, sign * lift);
    bone(humanoid, `${side}LowerArm`)?.rotation.set(0, sign * elbows, 0);
  }
  const chest = bone(humanoid, 'upperChest') ?? bone(humanoid, 'chest') ?? bone(humanoid, 'spine');
  chest?.rotation.set(-breath * CHEST_BREATH, 0, 0);
  const sway = wiggleAt(wiggle);
  bone(humanoid, 'hips')?.rotation.set(0, sway * WIGGLE_HIPS, 0);
  bone(humanoid, 'head')?.rotation.set(0, 0, -sway * WIGGLE_HEAD);
}
