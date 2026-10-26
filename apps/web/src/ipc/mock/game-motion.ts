/**
 * Motion in mock mode for a game's imported models, which have no bundled
 * fixture: a rig and clips made up from the numbers the import read
 * (`joints`, `animations`), with the long clip names hand rigged game
 * characters carry.
 */
import type { Asset, Clip } from '../client';
import type { MotionFixture } from './motion-fixtures';

const GROUPS = ['Locomotion', 'Combat', 'Spellcast', 'Reaction', 'Emote', 'Traversal'];
const ACTIONS = [
  'Idle_Breathing_Loop',
  'Walk_Forward_Relaxed',
  'Run_Forward_Combat_Stance',
  'Sprint_Start_Left_Foot',
  'Strafe_Left_Guard_Up',
  'Light_Attack_Combo_Chain_03',
  'Heavy_Attack_Charged_Release',
  'Parry_Riposte_Overhead',
  'Dodge_Roll_Backward_Recover',
  'Hit_React_Front_Stagger',
  'Cast_Frost_Lance_Forward',
  'Death_Fall_Backward_Long',
];

/** The clip names a heavily animated character carries, `count` of them. */
export function clipNames(count: number): string[] {
  return Array.from({ length: count }, (_, i) => {
    const group = GROUPS[i % GROUPS.length];
    const action = ACTIONS[Math.floor(i / GROUPS.length) % ACTIONS.length];
    const take = String(Math.floor(i / (GROUPS.length * ACTIONS.length)) + 1).padStart(2, '0');
    return `${group}_${action}_${take}`;
  });
}

function clip(name: string, index: number): Clip {
  const frames = 24 + ((index * 37) % 160);
  return {
    name,
    index,
    duration: frames / 30,
    keys: frames + 1,
    frames,
    fps: 30,
    channels: 360,
    targets: 119,
    bytes: 40_000 + ((index * 7_919) % 90_000),
    shippedBytes: null,
    rootTravel: name.includes('Idle') ? null : 0.4 + (index % 5) * 0.6,
    loops: name.includes('Loop'),
  };
}

/** A made up rig and clips for an imported model the import found bones in. */
export function importedMotion(asset: Asset | undefined): MotionFixture | null {
  const stats = asset?.versions[asset.versions.length - 1]?.stats;
  if (!stats || stats.joints === 0) return null;
  const jointNames = Array.from({ length: stats.joints }, (_, i) => `DEF-bone.${i}`);
  return {
    rig: {
      joints: stats.joints,
      jointNames,
      kind: 'other',
      humanoid: false,
      map: [],
      maxInfluences: 4,
    },
    clips: clipNames(stats.animations).map(clip),
    parts: [asset.name],
  };
}
