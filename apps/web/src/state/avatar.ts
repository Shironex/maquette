/**
 * The Avatar view's words and numbers: the VRM licence choices, the bone
 * map grouped for reading, the rest pose measures before and after a
 * normalise, what a reweight moved off the arms, and the checks a meta
 * must pass before a build writes.
 */
import type {
  AvatarBuild,
  BoneNaming,
  AvatarDefaults,
  AvatarPermission,
  AvatarSpec_Serialize,
  AvatarView_Serialize,
  CommercialUsage,
  CreditNotation,
  MaterialMode,
  Modification,
  NormalizeReport,
  PoseMeasures,
  ProjectManifest,
  ReweightReport,
  VrmBone,
  VrmMeta_Serialize,
  VrmReadiness,
} from '@/ipc/client';

/** VRM 1.0's own licence; any other licence goes in the other licence URL. */
export const LICENSE_URL = 'https://vrm.dev/licenses/1.0/';

export interface Choice<T extends string> {
  value: T;
  label: string;
}

export const PERMISSIONS: readonly Choice<AvatarPermission>[] = [
  { value: 'onlyAuthor', label: 'Only the author' },
  { value: 'onlySeparatelyLicensedPerson', label: 'People the author licenses' },
  { value: 'everyone', label: 'Everyone' },
];

export const COMMERCIAL: readonly Choice<CommercialUsage>[] = [
  { value: 'personalNonProfit', label: 'Personal, non profit' },
  { value: 'personalProfit', label: 'Personal, for profit' },
  { value: 'corporation', label: 'Companies too' },
];

export const CREDIT: readonly Choice<CreditNotation>[] = [
  { value: 'required', label: 'Credit required' },
  { value: 'unnecessary', label: 'No credit needed' },
];

export const MODIFICATION: readonly Choice<Modification>[] = [
  { value: 'prohibited', label: 'No changes' },
  { value: 'allowModification', label: 'Changes allowed' },
  { value: 'allowModificationRedistribution', label: 'Changes, and sharing them' },
];

export const MATERIALS: readonly (Choice<MaterialMode> & { hint: string })[] = [
  {
    value: 'mtoon-soft',
    label: 'MToon soft',
    hint: 'Toon shading from the base color, no outline. Looks alike in every VRM app.',
  },
  { value: 'pbr', label: 'PBR', hint: "The model's own materials as they are." },
];

/** The yes or no licence flags, in the order VRM lists them. */
export type UsageFlag =
  | 'allowRedistribution'
  | 'allowExcessivelyViolentUsage'
  | 'allowExcessivelySexualUsage'
  | 'allowPoliticalOrReligiousUsage'
  | 'allowAntisocialOrHateUsage';

export const USAGE_FLAGS: readonly { key: UsageFlag; label: string }[] = [
  { key: 'allowRedistribution', label: 'Others may share the file' },
  { key: 'allowExcessivelyViolentUsage', label: 'Violent use' },
  { key: 'allowExcessivelySexualUsage', label: 'Sexual use' },
  { key: 'allowPoliticalOrReligiousUsage', label: 'Political or religious use' },
  { key: 'allowAntisocialOrHateUsage', label: 'Antisocial or hateful use' },
];

/** A new avatar's spec from the workspace defaults, as the app makes it. */
export function specFromDefaults(
  defaults: AvatarDefaults | undefined,
  name: string
): AvatarSpec_Serialize {
  const d = defaults ?? {};
  const author = (d.author ?? '').trim();
  return {
    meta: {
      name: name.trim(),
      authors: author ? [author] : [],
      contactInformation: d.contactInformation ?? null,
      copyrightInformation: d.copyrightInformation ?? null,
      licenseUrl: LICENSE_URL,
      avatarPermission: d.avatarPermission ?? 'onlyAuthor',
      commercialUsage: d.commercialUsage ?? 'personalNonProfit',
      creditNotation: d.creditNotation ?? 'required',
      modification: d.modification ?? 'prohibited',
      allowRedistribution: d.allowRedistribution ?? false,
      allowExcessivelyViolentUsage: d.allowExcessivelyViolentUsage ?? false,
      allowExcessivelySexualUsage: d.allowExcessivelySexualUsage ?? false,
      allowPoliticalOrReligiousUsage: d.allowPoliticalOrReligiousUsage ?? false,
      allowAntisocialOrHateUsage: d.allowAntisocialOrHateUsage ?? false,
      otherLicenseUrl: d.otherLicenseUrl ?? null,
    },
    bones: {},
    material: d.material ?? 'mtoon-soft',
  };
}

/** What a build would refuse in the meta, as the Rust writer says it. */
export function metaProblem(meta: VrmMeta_Serialize): string | null {
  if (!meta.name.trim()) return 'The avatar needs a name.';
  if (meta.authors.length === 0) return 'Name at least one author.';
  if (meta.authors.some(a => !a.trim())) {
    return 'An author name is blank. Fill it in or remove it.';
  }
  return null;
}

/** "Ana, Studio North" to two authors; blanks between commas are dropped. */
export function parseAuthors(text: string): string[] {
  return text
    .split(',')
    .map(a => a.trim())
    .filter(Boolean);
}

/** `leftUpperArm` to "Left upper arm". */
export function boneLabel(bone: string): string {
  const words = bone.replace(/([A-Z])/g, ' $1').toLowerCase();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

const FINGER = /(Thumb|Index|Middle|Ring|Little)/;

export interface BoneGroup {
  id: 'required' | 'body' | 'fingers';
  label: string;
  bones: VrmBone[];
  mapped: number;
}

/** The 55 bones as the view reads them: the 15 VRM needs, the rest of the body, the fingers. */
export function boneGroups(readiness: VrmReadiness): BoneGroup[] {
  const group = (id: BoneGroup['id'], label: string, bones: VrmBone[]): BoneGroup => ({
    id,
    label,
    bones,
    mapped: bones.filter(b => b.joint).length,
  });
  const optional = readiness.bones.filter(b => !b.required);
  return [
    group(
      'required',
      'Required',
      readiness.bones.filter(b => b.required)
    ),
    group(
      'body',
      'Optional body',
      optional.filter(b => !FINGER.test(b.bone))
    ),
    group(
      'fingers',
      'Fingers',
      optional.filter(b => FINGER.test(b.bone))
    ),
  ];
}

const NAMING_LABEL: Record<BoneNaming, string> = {
  vrm: 'VRM',
  mixamo: 'Mixamo',
  unreal: 'Unreal',
  tripo: 'Tripo',
  descriptive: 'descriptive',
  override: 'set by you',
};

/** "Mixamo", or "descriptive" for names read as words. */
export function namingLabel(naming: BoneNaming): string {
  return NAMING_LABEL[naming];
}

/**
 * Where a bone's mapping comes from, as its row says it: set by you, left
 * out by you, auto with the naming it matched by, or nothing.
 */
export function boneSource(bone: VrmBone, overrides: Partial<Record<string, string | null>>) {
  if (bone.bone in overrides) {
    return overrides[bone.bone] ? 'set by you' : 'left out by you';
  }
  if (bone.joint && bone.naming) return `auto · ${namingLabel(bone.naming)}`;
  return null;
}

/** One choice in a bone's joint picker. */
export interface JointChoice {
  /** A joint name; null leaves the bone unmapped, undefined maps it by name. */
  value: string | null | undefined;
  id: string;
  label: string;
  /** The bone the joint maps to now, other than this one. */
  takenBy: string | null;
}

/**
 * The joints a bone may map to, filtered by `query`: Automatic and
 * Leave unmapped first while the search is empty, then every joint whose
 * name holds the query, in the rig's order.
 */
export function jointChoices(readiness: VrmReadiness, bone: string, query: string): JointChoice[] {
  const q = query.trim().toLowerCase();
  const holder = new Map(readiness.bones.filter(b => b.joint).map(b => [b.joint!, b.bone]));
  const fixed: JointChoice[] = q
    ? []
    : [
        { value: undefined, id: 'auto', label: 'Automatic, by name', takenBy: null },
        { value: null, id: 'none', label: 'Leave unmapped', takenBy: null },
      ];
  const joints = readiness.joints
    .map((j, i): JointChoice => {
      const taken = holder.get(j);
      return {
        value: j,
        id: `joint-${i}`,
        label: j,
        takenBy: taken && taken !== bone ? taken : null,
      };
    })
    .filter(c => c.label.toLowerCase().includes(q));
  return [...fixed, ...joints];
}

export interface MeasureRow {
  id: string;
  label: string;
  before: string;
  after: string | null;
  /** The after value is what VRM wants. */
  fixed: boolean;
}

const deg = (v: number | null | undefined) =>
  v === null || v === undefined ? 'n/a' : `${Math.round(Math.abs(v))}°`;
const cm = (v: number | null | undefined) =>
  v === null || v === undefined ? 'n/a' : `${Math.round(Math.abs(v) * 100)} cm`;
const metres = (v: number | null | undefined) =>
  v === null || v === undefined ? 'n/a' : `${v.toFixed(2)} m`;

/** Near enough to VRM's rest pose to count as there. */
const near = (v: number | null | undefined, within: number) =>
  v !== null && v !== undefined && Math.abs(v) <= within;

/**
 * The rest pose numbers before a normalise and after it: the facing off
 * +Z, each arm off level, each elbow's bend, the soles off the ground and
 * the hips off centre. `after` is null until a normalise ran; a measure
 * missing on both sides is left out.
 */
export function measureRows(
  before: PoseMeasures | null,
  after: PoseMeasures | null = null
): MeasureRow[] {
  const row = (
    id: string,
    label: string,
    pick: (m: PoseMeasures) => number | null | undefined,
    show: (v: number | null | undefined) => string,
    within: number
  ): MeasureRow => ({
    id,
    label,
    before: before ? show(pick(before)) : 'n/a',
    after: after ? show(pick(after)) : null,
    fixed: after ? near(pick(after), within) : false,
  });
  const rows = [
    row('facing', 'Facing off +Z', m => m.facingDeg, deg, 1),
    row('arm-left', 'Left arm off level', m => m.armLeftDeg, deg, 3),
    row('arm-right', 'Right arm off level', m => m.armRightDeg, deg, 3),
    row('elbow-left', 'Left elbow bend', m => m.elbowLeftDeg, deg, 3),
    row('elbow-right', 'Right elbow bend', m => m.elbowRightDeg, deg, 3),
    row('ground', 'Soles off the ground', m => m.groundY, cm, 0.01),
    row('hips', 'Hips off centre', m => m.hipsOffset, cm, 0.02),
    row('height', 'Height', m => m.height, metres, Infinity),
  ];
  // A measure the reader could not take on either side says nothing.
  return rows.filter(r => r.before !== 'n/a' || (r.after !== null && r.after !== 'n/a'));
}

/** The pose readiness measured on the model as it is, in the script's terms. */
export function measuresOf(readiness: VrmReadiness | null): PoseMeasures | null {
  if (!readiness) return null;
  const p = readiness.pose;
  return {
    facingDeg: p.facingDeg,
    armLeftDeg: p.armLeftDeg,
    armRightDeg: p.armRightDeg,
    groundY: p.groundY,
    hipsOffset: p.hipsOffset,
    height: p.height,
  };
}

/** "Turned 90°, lifted the left arm 55° and the right arm 52°, elbows straightened." */
export function poseSummary(report: NormalizeReport): string {
  const parts: string[] = [];
  const turned = report.turnedDeg ?? 0;
  if (Math.abs(turned) >= 0.5) parts.push(`turned ${deg(turned)}`);
  const lifts = Object.entries(report.lifted ?? {})
    .filter(([, lift]) => lift.shoulderDeg !== null && lift.shoulderDeg !== undefined)
    .map(([side, lift]) => `the ${side} arm ${deg(lift.shoulderDeg)}`);
  if (lifts.length > 0) parts.push(`lifted ${lifts.join(' and ')}`);
  parts.push(report.elbows === 'keep' ? 'elbows kept' : 'elbows straightened');
  const text = parts.join(', ');
  return `${text.charAt(0).toUpperCase()}${text.slice(1)}.`;
}

const count = (n: number | undefined) => (n ?? 0).toLocaleString('en-US');

/** What a reweight moved, in a sentence: how many vertices, and from which arm bones most. */
export function reweightSummary(report: ReweightReport): string {
  const from = Object.entries(report.fromBones ?? {})
    .sort((a, b) => b[1] - a[1])
    .slice(0, 2)
    .map(([bone]) => bone);
  const most = from.length > 0 ? `, most from ${from.join(' and ')}` : '';
  return (
    `Moved ${count(report.moved)} vertices of cloth and hair to the trunk${most}; ` +
    `kept ${count(report.keptAsSleeve)} on the arms as sleeve.`
  );
}

/** What a build answered, for the notice after it. */
export function buildNotice(built: AvatarBuild): { title: string; detail: string } {
  if (built.status === 'reweight waiting for approval') {
    return {
      title: 'The freed arms wait for you',
      detail: 'Look at the before and after sheets, then approve or deny the new weights.',
    };
  }
  if (built.status === 'written') {
    const left = built.export.unresolved.filter(f => f.level !== 'note').length;
    return {
      title: 'VRM written',
      detail:
        left === 0
          ? 'Nothing left to fix.'
          : `${left} ${left === 1 ? 'thing' : 'things'} still to look at; see the result.`,
    };
  }
  if (built.status === 'denied') {
    return {
      title: 'This rest pose was turned down',
      detail: 'Change an option, like keeping the elbows, and build again.',
    };
  }
  return {
    title: 'The new rest pose waits for you',
    detail: 'Look at the before and after sheets, then approve or deny it.',
  };
}

/** The project's character assets, the ones an avatar is built from. */
export function characters(project: ProjectManifest | null | undefined) {
  return (project?.assets ?? []).filter(a => a.kind === 'character' && a.versions.length > 0);
}

export interface BuildAction {
  label: string;
  disabled: boolean;
  tooltip: string;
}

/** The Build button: what it says, and why it cannot run yet. */
export function buildAction(
  view: AvatarView_Serialize | null,
  draft: AvatarSpec_Serialize | null
): BuildAction {
  const label = view?.export?.exists ? 'Build again' : 'Build VRM';
  const off = (tooltip: string) => ({ label, disabled: true, tooltip });
  if (!view || !draft) return off('Pick a character first');
  if (!view.readiness) return off(view.readinessError ?? 'The model has no humanoid rig');
  const problem = metaProblem(draft.meta);
  if (problem) return off(problem.replace(/\.$/, ''));
  const weights = view.reweighted?.state;
  if (weights === 'pending') return off('Approve or deny the new weights first');
  const pose = view.normalized?.state;
  if (pose === 'pending') return off('Approve or deny the new rest pose first');
  const file = `exports/${view.asset}.vrm`;
  return {
    label,
    disabled: false,
    tooltip:
      view.needsReweight && !weights
        ? 'Frees cloth and hair from the arms headless in Blender, then waits for your yes'
        : view.needsNormalize && pose !== 'approved'
          ? 'Normalises the rest pose headless in Blender, then waits for your yes'
          : `Writes ${file}, never a new version`,
  };
}
