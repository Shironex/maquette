/**
 * Compare two versions of an asset, whoever made them: what each cost, how
 * it measures and how sharp its texture reads at a camera stop, for the same
 * table and dots as before and after a split.
 */
import type { InspectModel, ModelParts, ProviderJob, RealSize, SizeAxis } from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';
import { num, type Stop } from './inspector';
import { PROVIDER_LABEL } from './providers';
import { betterOf, checksLine, checksTrouble, type CompareRow } from './parts-compare';

/** What is known about one side. */
export interface VersionFacts {
  version: number;
  numbers: ModelParts | null;
  model: InspectModel | null;
  /** The send that made it; a bake back or an assembly has none. */
  job: ProviderJob | null;
}

/** Where the numbers are judged from: a stop, or the model filling the view. */
export interface CompareAt {
  name: string;
  /** How tall the model stands on a 1080 line view. */
  pixels: number;
}

/** The screen height stops are judged on. */
const VIEW_HEIGHT = 1080;
/** Texels per screen pixel from which a texture reads sharp, and still fine. */
const SHARP = 1.5;
const FINE = 0.75;

const AXIS: Record<Exclude<SizeAxis, 'longest'>, number> = { width: 0, height: 1, depth: 2 };

/** The send that made a version of an asset. */
export function jobForVersion(
  jobs: readonly ProviderJob[],
  asset: string,
  version: number
): ProviderJob | null {
  return jobs.find(j => j.asset === asset && j.version === version) ?? null;
}

/** "Tripo v3", or just "v3" for a version no send made. */
export function versionTitle(facts: Pick<VersionFacts, 'version' | 'job'>): string {
  return facts.job
    ? `${PROVIDER_LABEL[facts.job.provider]} v${facts.version}`
    : `v${facts.version}`;
}

/** What the send cost: what the provider reported, else its estimate. */
export function creditsOf(job: ProviderJob | null): number | null {
  return job?.credits ?? job?.estimate ?? null;
}

/** "30 credits", or null when no send made it. */
export function creditsNote(job: ProviderJob | null): string | null {
  const credits = creditsOf(job);
  return credits === null ? null : `${credits} credits`;
}

/** The model's longest side in metres, 0 when unmeasured. */
export const longestSide = (model: InspectModel | null): number =>
  Math.max(0, ...(model?.size ?? []).map(n => n ?? 0));

/** How tall a model of `extent` metres at `place` stands at a stop, in pixels. */
export function pixelsAt(extent: number, place: readonly number[], stop: Stop): number {
  const distance = Math.hypot(
    stop.position[0] - (place[0] ?? 0),
    stop.position[1] - (place[1] ?? 0),
    stop.position[2] - (place[2] ?? 0)
  );
  if (distance <= 0) return VIEW_HEIGHT;
  const seen = 2 * distance * Math.tan((stop.fov * Math.PI) / 360);
  return Math.round((extent / seen) * VIEW_HEIGHT);
}

/** Whether a stop's camera has `place` in its view, wide enough for a 16 by 10 frame. */
export function seesPlace(stop: Stop, place: readonly number[]): boolean {
  const to = [0, 1, 2].map(i => (place[i] ?? 0) - stop.position[i]!);
  const look = [0, 1, 2].map(i => stop.target[i]! - stop.position[i]!);
  const lengths = Math.hypot(...to) * Math.hypot(...look);
  if (lengths <= 0) return false;
  const cos = (to[0]! * look[0]! + to[1]! * look[1]! + to[2]! * look[2]!) / lengths;
  return Math.acos(Math.min(1, cos)) <= ((stop.fov / 2) * 1.6 * Math.PI) / 180;
}

/** The index of the stop where the model stands biggest, or null when none sees it. */
export function biggestStop(stops: readonly Stop[], place: readonly number[], extent: number) {
  let best: number | null = null;
  let most = -1;
  for (const [i, stop] of stops.entries()) {
    const pixels = seesPlace(stop, place) ? pixelsAt(extent, place, stop) : -1;
    if (pixels > most) {
      most = pixels;
      best = i;
    }
  }
  return best;
}

/** How much of the real size a model has, 0 to 1; null without one to match. */
export function realSizeMatch(model: InspectModel | null, size: RealSize | null): number | null {
  if (!model || !size) return null;
  const measured =
    size.axis === 'longest' ? longestSide(model) : (model.size[AXIS[size.axis]] ?? 0);
  const real = num(size.value) * (size.unit === 'cm' ? 0.01 : 1);
  if (measured <= 0 || real <= 0) return null;
  return Math.min(measured, real) / Math.max(measured, real);
}

/**
 * Texels per screen pixel: an estimate from the longest texture edge against
 * how tall the model stands, since the real spread depends on the UVs.
 */
function texelsPerPixel(facts: VersionFacts, at: CompareAt | null): number | null {
  const edge = facts.numbers?.maxTexture ?? 0;
  if (edge <= 0) return null;
  return edge / Math.max(1, at?.pixels ?? VIEW_HEIGHT);
}

function detailWord(texels: number | null): string {
  if (texels === null) return 'no texture';
  return texels >= SHARP ? 'sharp' : texels >= FINE ? 'fine' : 'soft';
}

/** Only numbers both sides have can be better than each other. */
function known(left: number | null, right: number | null, higher = false) {
  return left === null || right === null ? null : betterOf(left, right, higher);
}

const orDash = (value: string | null) => value ?? '-';

/** The table under two versions. */
export function versionRows(
  left: VersionFacts,
  right: VersionFacts,
  at: CompareAt | null,
  size: RealSize | null
): CompareRow[] {
  const row = (
    label: string,
    read: (f: VersionFacts) => number | null,
    text: (n: number) => string
  ): CompareRow => {
    const [l, r] = [read(left), read(right)];
    return {
      label,
      left: l === null ? '-' : text(l),
      right: r === null ? '-' : text(r),
      better: known(l, r),
    };
  };
  const [matchLeft, matchRight] = [
    realSizeMatch(left.model, size),
    realSizeMatch(right.model, size),
  ];
  const percent = (m: number | null) => (m === null ? null : `${Math.round(m * 100)}%`);
  const [texelsLeft, texelsRight] = [texelsPerPixel(left, at), texelsPerPixel(right, at)];
  const sameWord = detailWord(texelsLeft) === detailWord(texelsRight);
  const noChecks = (f: VersionFacts) => (f.numbers ? checksTrouble(f.numbers) : null);
  const [creditsLeft, creditsRight] = [creditsOf(left.job), creditsOf(right.job)];
  return [
    row('Triangles', f => f.numbers?.triangles ?? f.model?.triangles ?? null, formatK),
    row('Draw calls', f => f.numbers?.drawCalls ?? null, String),
    row('GPU memory', f => f.model?.gpuBytes ?? null, formatBytes),
    row('Transfer', f => f.numbers?.bytes ?? f.model?.bytes ?? null, formatBytes),
    {
      label: 'Real size match',
      left: size ? orDash(percent(matchLeft)) : 'no real size set',
      right: size ? orDash(percent(matchRight)) : 'no real size set',
      better: known(matchLeft, matchRight, true),
    },
    {
      label: at ? `Texture detail at ${at.name}` : 'Texture detail, close up',
      left: detailWord(texelsLeft),
      right: detailWord(texelsRight),
      better: sameWord ? null : known(texelsLeft ?? 0, texelsRight ?? 0, true),
    },
    {
      label: 'Inbox checks',
      left: left.numbers ? checksLine(left.numbers) : '-',
      right: right.numbers ? checksLine(right.numbers) : '-',
      better: known(noChecks(left), noChecks(right)),
    },
    {
      label: 'Credits',
      left: left.job ? String(creditsLeft ?? '-') : 'no send',
      right: right.job ? String(creditsRight ?? '-') : 'no send',
      better: known(creditsLeft, creditsRight),
    },
  ];
}
