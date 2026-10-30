/**
 * The Springs tab's words and numbers: which candidates are confirmed in a
 * spec, the joints and preset each gets, and what the chain step added.
 * Nothing is confirmed on its own: a person picks each part.
 */
import type {
  AvatarSpec_Serialize,
  Chained,
  SpringCandidate,
  SpringKind,
  SpringPreset,
  SpringSpec,
} from '@/ipc/client';

export const KIND_LABEL: Record<SpringKind, string> = {
  hair: 'Hair',
  cloth: 'Cloth',
  accessory: 'Accessory',
};

/** The presets, each with what it suits. */
export const PRESETS: readonly { value: SpringPreset; label: string; hint: string }[] = [
  { value: 'soft', label: 'Soft', hint: 'Light drag, little gravity: hair' },
  { value: 'heavy', label: 'Heavy', hint: 'More drag and gravity: cloth' },
  { value: 'stiff', label: 'Stiff', hint: 'Springs back quickly: charms and small parts' },
];

/** The preset a kind swings with unless one is picked. */
export const KIND_PRESET: Record<SpringKind, SpringPreset> = {
  hair: 'soft',
  cloth: 'heavy',
  accessory: 'stiff',
};

/** Joints a chain may have. */
export const JOINTS_MIN = 1;
export const JOINTS_MAX = 16;

/** The springs a spec confirms. */
export function confirmedSprings(spec: AvatarSpec_Serialize | null): SpringSpec[] {
  return spec?.springs ?? [];
}

/** A candidate confirmed as it was found: its line, its bone, its kind's preset. */
export function specOf(candidate: SpringCandidate): SpringSpec {
  return {
    id: candidate.id,
    kind: candidate.kind,
    rootBone: candidate.rootBone,
    line: candidate.line,
    joints: null,
    preset: null,
  };
}

/** `spec` with `candidate` confirmed or no longer confirmed. */
export function toggleSpring(
  spec: AvatarSpec_Serialize,
  candidate: SpringCandidate,
  on: boolean
): AvatarSpec_Serialize {
  const rest = confirmedSprings(spec).filter(s => s.id !== candidate.id);
  return { ...spec, springs: on ? [...rest, specOf(candidate)] : rest };
}

/** `spec` with the confirmed spring `id` changed by `change`. */
export function editSpring(
  spec: AvatarSpec_Serialize,
  id: string,
  change: Partial<Pick<SpringSpec, 'joints' | 'preset'>>
): AvatarSpec_Serialize {
  return {
    ...spec,
    springs: confirmedSprings(spec).map(s => (s.id === id ? { ...s, ...change } : s)),
  };
}

/** "92%" sure. */
export function confidenceLabel(confidence: number): string {
  return `${Math.round(confidence * 100)}%`;
}

/** "1.04 m" or "18 cm". */
export function lengthLabel(metres: number): string {
  return metres >= 1 ? `${metres.toFixed(2)} m` : `${Math.round(metres * 100)} cm`;
}

/** What the chain step added, in a sentence. */
export function chainSummary(chained: Chained): string {
  const chains = chained.report.chains ?? [];
  const joints = chains.reduce((n, c) => n + (c.joints?.length ?? 0), 0);
  const colliders = chained.report.colliders?.length ?? 0;
  const what = chains.map(c => c.id).join(', ');
  return (
    `${chains.length} ${chains.length === 1 ? 'chain' : 'chains'} (${what}) with ${joints} joints, ` +
    `${(chained.report.changed ?? 0).toLocaleString('en-US')} vertices reweighted, ` +
    `${colliders} colliders from the body.`
  );
}

/** The notice after a build whose chains wait for a yes. */
export const SPRINGS_WAITING = {
  title: 'The spring chains wait for you',
  detail: 'Look at the parts and the sway sheet, then approve or deny the chains.',
};
