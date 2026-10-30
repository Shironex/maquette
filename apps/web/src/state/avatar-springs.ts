/**
 * The Springs tab's words and numbers: which candidates are confirmed in a
 * spec (or that none are), the joints and preset each gets, and what the
 * chain step added. Nothing is confirmed on its own: a person picks each
 * part, or confirms none.
 */
import type {
  AvatarBuild,
  AvatarSpec_Serialize,
  Chained,
  SpringCandidate,
  SpringCarried,
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

/**
 * What the person chose: some springs, none at all, or nothing yet (a
 * build then waits for a pick when the last export carried springs).
 */
export type SpringsChoice = 'picked' | 'none' | 'unpicked';

export function springsChoice(spec: AvatarSpec_Serialize | null): SpringsChoice {
  if (confirmedSprings(spec).length > 0) return 'picked';
  return spec?.noSprings ? 'none' : 'unpicked';
}

/** `spec` confirming that the VRM carries no springs, or taking that back. */
export function confirmNoSprings(spec: AvatarSpec_Serialize, on: boolean): AvatarSpec_Serialize {
  return { ...spec, springs: on ? [] : confirmedSprings(spec), noSprings: on };
}

/** The confirmed springs no candidate goes by now: the model changed under them. */
export function unmatchedSprings(
  spec: AvatarSpec_Serialize | null,
  candidates: readonly SpringCandidate[]
): SpringSpec[] {
  return confirmedSprings(spec).filter(s => !candidates.some(c => c.id === s.id));
}

/** A candidate confirmed as it was found: its line, its bone, its kind's preset. */
export function specOf(candidate: SpringCandidate): SpringSpec {
  return {
    id: candidate.id,
    kind: candidate.kind,
    rootBone: candidate.rootBone,
    line: candidate.line,
    // A small piece keeps its short chain; a longer part gets its joints by length.
    joints: candidate.small ? candidate.joints : null,
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
  if (!on) return { ...spec, springs: rest };
  return { ...spec, springs: [...rest, specOf(candidate)], noSprings: false };
}

/** `spec` without the confirmed spring `id`. */
export function dropSpring(spec: AvatarSpec_Serialize, id: string): AvatarSpec_Serialize {
  return { ...spec, springs: confirmedSprings(spec).filter(s => s.id !== id) };
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

/** The notice after a build that waits for a pick. */
export const SPRINGS_UNPICKED = {
  title: 'Pick the springs first',
  detail:
    'The last VRM had springs and none are confirmed now. Pick the parts to keep on the Springs tab, or confirm no springs; nothing was written.',
};

/** The notice after a build whose confirmed springs match no part now. */
export function springsStaleNotice(stale: readonly string[]): { title: string; detail: string } {
  return {
    title: 'The confirmed springs no longer match; pick again',
    detail: `No part found on this model matches ${stale.join(', ')} clearly enough to carry it over. Pick again on the Springs tab; nothing was written.`,
  };
}

/** "cloth-1 is hair-1 now", for a build that carried picks over. */
export function carriedSummary(carried: readonly SpringCarried[]): string {
  return carried
    .map(c => (c.from === c.to ? `${c.from} was found again` : `${c.from} is ${c.to} now`))
    .join(', ');
}

/** The notice after a build the springs stopped or hold, if they did. */
export function springsBuildNotice(built: AvatarBuild): { title: string; detail: string } | null {
  if (built.status === 'candidates waiting for a pick') return SPRINGS_UNPICKED;
  if (built.status === 'springs no longer match') return springsStaleNotice(built.stale);
  if (built.status !== 'springs waiting for approval') return null;
  if (built.carried.length === 0) return SPRINGS_WAITING;
  return {
    title: SPRINGS_WAITING.title,
    detail: `The model changed under the confirmed springs, so they were carried over: ${carriedSummary(built.carried)}. ${SPRINGS_WAITING.detail}`,
  };
}
