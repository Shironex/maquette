/**
 * The mock bake's seams, as the core finds and judges them (recipes/seams.rs,
 * bake/seams.rs): each join and each freeze or hold over easing in and out on
 * the timeline, with made up steps a review can show. A join pops while its
 * blend is shorter than 10 frames (the extra hip move is spread over the
 * window, so a wider blend fixes it), a hold seam passes. A two handed grip's
 * hands keep within 0.8 cm, and a mirror swaps every pair. Each seam is held to
 * the clips around it, never to a near seam, so two near joins each pop alone;
 * a seam with no frame around it outside the seams is not measured.
 */
import type {
  RecipeBakeResult,
  RecipeDoc,
  RecipeGripResult,
  RecipeMirrorResult,
  RecipeReviewLine,
  SeamCheck,
} from '@/ipc/client';
import { clipsOf, timeline, type Timeline } from '@/state/recipe-timeline';

/** The core's limits: hips and any bone past the clips around them in one frame. */
export const SEAM_HIPS_CM = 2;
export const SEAM_TURN_DEGREES = 25;
export const GRIP_GAP_LIMIT_CM = 2;
/** The extra hip move a mock join spreads over its window, cm frames. */
const JOIN_EXTRA_CM = 24.8;
/** The hips' walk each frame away from any join, cm. */
const HIPS_WALK_CM = 1.2;
/** How many frames either side of a seam stand for the clips around it. */
const AROUND_FRAMES = 6;

type Window = Pick<SeamCheck, 'kind' | 'clip' | 'at' | 'freeze' | 'first' | 'last'>;

/** Every seam of `doc` on timeline `t`, in frame order, as the core lays them. */
export function windowsOf(doc: RecipeDoc, t: Timeline): Window[] {
  const out: Window[] = [];
  const push = (w: Window) => {
    const first = Math.max(w.first, 1);
    const last = Math.min(w.last, t.frames);
    if (first <= last) out.push({ ...w, first, last });
  };
  for (const s of t.seams)
    push({
      kind: 'join',
      clip: s.clip,
      at: null,
      freeze: false,
      first: s.frame,
      last: s.frame + s.frames,
    });
  for (const edit of doc.recipe.time ?? []) {
    if (edit.kind === 'retime' || edit.at == null) continue;
    const held: [number, number][] = [];
    t.keys.forEach((key, i) => {
      const w = [key.from, key.to]
        .filter(s => s?.clip === edit.clip && s.hold?.at === edit.at)
        .reduce((m, s) => Math.max(m, s?.hold?.weight ?? 0), 0);
      if (w > 0) held.push([i, w]);
    });
    if (!held.length) continue;
    const hFirst = held[0]![0];
    const hLast = held[held.length - 1]![0];
    const whole = held.filter(([, w]) => w >= 1 - 1e-6).map(([i]) => i);
    const fullFirst = whole[0] ?? hFirst;
    const fullLast = whole[whole.length - 1] ?? hLast;
    const freeze = edit.kind === 'freeze';
    const base = { clip: edit.clip, at: edit.at, freeze };
    push({ ...base, kind: 'hold-in', first: hFirst, last: Math.max(fullFirst, hFirst) });
    push({ ...base, kind: 'hold-out', first: fullLast + 1, last: hLast + 1 });
  }
  return out.sort((a, b) => a.first - b.first || a.last - b.last);
}

const round = (n: number) => Math.round(n * 10) / 10;

/** The steps either side of `w` within `frames`, none inside another seam, as the core takes them. */
function aroundOf(w: Window, all: Window[], frames: number): number[] {
  const out: number[] = [];
  for (let i = Math.max(w.first - AROUND_FRAMES, 1); i < w.first; i++) out.push(i);
  for (let i = w.last + 1; i <= Math.min(w.last + AROUND_FRAMES, frames); i++) out.push(i);
  return out.filter(i => !all.some(o => o.first <= i && i <= o.last));
}

/** Each seam of `doc` as a fake bake measures it. */
export function seamsOf(doc: RecipeDoc): SeamCheck[] {
  const recipe = doc.recipe;
  const clips = clipsOf(recipe);
  const kept = (i: number) => {
    const c = clips[i];
    return c ? (c.trim ? c.trim[1] - c.trim[0] : c.frames) : 0;
  };
  const t = timeline(recipe);
  const all = windowsOf(doc, t);
  const extraOf = (w: Window) =>
    w.kind === 'join' ? JOIN_EXTRA_CM / Math.max(clips[w.clip]?.blend?.frames ?? 0, 1) : 0;
  /** The hips' step into frame `i`: the walk, plus the extra of a join it sits in. */
  const stepAt = (i: number) =>
    HIPS_WALK_CM +
    all.filter(o => o.first <= i && i <= o.last).reduce((m, o) => Math.max(m, extraOf(o)), 0);
  return all.map(w => {
    const join = w.kind === 'join';
    const blend = join ? (clips[w.clip]?.blend?.frames ?? 0) : null;
    const around = aroundOf(w, all, t.frames);
    const hips = HIPS_WALK_CM + extraOf(w);
    const hipsAround = around.reduce((m, i) => Math.max(m, stepAt(i)), 0);
    const hipsOver = round(Math.max(hips - hipsAround, 0));
    const measured = around.length > 0;
    const edit = join
      ? null
      : (recipe.time ?? []).find(e => e.kind !== 'retime' && e.clip === w.clip && e.at === w.at);
    const check: SeamCheck = {
      ...w,
      frame: join ? w.first + Math.floor((w.last - w.first) / 2) : w.last,
      hipsCm: round(hips),
      hipsAroundCm: round(hipsAround),
      hipsOverCm: hipsOver,
      bone: 'chest',
      turnDegrees: join ? 6 : 9,
      turnAroundDegrees: join ? 4 : 3,
      turnOverDegrees: join ? 2 : 6,
      pops: measured && hipsOver > SEAM_HIPS_CM,
      about: join
        ? `${clips[w.clip - 1]?.name ?? ''} into ${clips[w.clip]?.name ?? ''}`
        : (clips[w.clip]?.name ?? ''),
      blend,
      blendMost: join ? Math.floor(Math.min(kept(w.clip), kept(w.clip - 1)) / 2) : null,
      ease: edit ? ((w.kind === 'hold-in' ? edit.easeIn : edit.easeOut) ?? 0) : null,
      easeMost: edit?.frames ?? null,
      measured,
    };
    return check;
  });
}

/** How far past its limits a seam is, the worse of the two (1 at a limit). */
export const ratioOf = (s: SeamCheck) =>
  Math.max((s.hipsOverCm ?? 0) / SEAM_HIPS_CM, (s.turnOverDegrees ?? 0) / SEAM_TURN_DEGREES);
/** Whether the hips (else a bone) go furthest past their limit. */
export const hipsWorse = (s: SeamCheck) =>
  (s.hipsOverCm ?? 0) / SEAM_HIPS_CM >= (s.turnOverDegrees ?? 0) / SEAM_TURN_DEGREES;

/** `grips` with a two handed grip's hand gap, as a fake bake measures it. */
export function withGaps(doc: RecipeDoc, grips: RecipeGripResult[]): RecipeGripResult[] {
  return grips.map(g => {
    const layer = doc.recipe.layers.find(l => l.id === g.layer);
    const bones = Object.keys(layer?.pose ?? {});
    const both = bones.some(b => b.startsWith('left')) && bones.some(b => b.startsWith('right'));
    return both && g.frames ? { ...g, gapCm: 0.8, gapFrame: g.frames[0] } : g;
  });
}

/** A mirrored recipe swaps every pair on the stand-in models. */
export const mirrorOf = (doc: RecipeDoc): RecipeMirrorResult | null =>
  doc.recipe.mirror ? { pairs: 26, unpaired: [] } : null;

/**
 * A seam line when nothing measured a seam (no hips motion there, or no
 * frames around to hold it to), as the core words it: `left` of `all` were not.
 */
const notMeasured = (
  id: string,
  label: string,
  s: SeamCheck,
  left: number,
  all: number
): RecipeReviewLine => ({
  id,
  label,
  about: s.about,
  value: 'not measured',
  limit: `${SEAM_HIPS_CM} cm or ${SEAM_TURN_DEGREES} deg`,
  measured: null,
  max: null,
  unit: null,
  status: 'not-measured',
  note:
    left === all
      ? 'The bake reported no hips motion there, or no frames around to hold it to, so no seam was measured.'
      : `${left} of ${all} seams were not measured (no hips motion there, or no frames around to hold them to); the rest stay within the limit.`,
});

/**
 * The line for `seams`: the worst measured seam when it pops, else not
 * measured while any seam was not, else the worst one's pass.
 */
function seamLine(id: string, label: string, seams: SeamCheck[], place: (s: SeamCheck) => string) {
  const first = seams[0];
  if (!first) return [];
  const judged = seams.filter(x => x.measured !== false).sort((a, b) => ratioOf(b) - ratioOf(a));
  const left = seams.length - judged.length;
  const s = judged[0];
  if (!s || (left > 0 && !s.pops)) {
    const unmeasured = seams.find(x => x.measured === false) ?? first;
    return [notMeasured(id, label, unmeasured, left, seams.length)];
  }
  const hips = hipsWorse(s);
  const more = judged.length - 1;
  const popping = judged.slice(1).filter(x => x.pops).length;
  const step = hips
    ? `Hips move ${(s.hipsCm ?? 0).toFixed(1)} cm in one frame; the clips around move ${(s.hipsAroundCm ?? 0).toFixed(1)} cm.`
    : `${s.bone ?? 'A bone'} turns ${(s.turnDegrees ?? 0).toFixed(0)} deg in one frame; around it, ${(s.turnAroundDegrees ?? 0).toFixed(0)} deg.`;
  const others = more
    ? ` ${more} more, ${popping ? `${popping} of them` : 'none'} past the limit.`
    : '';
  const line: RecipeReviewLine = {
    id,
    label,
    about: s.about,
    value: hips
      ? `${(s.hipsOverCm ?? 0).toFixed(1)} cm`
      : `${(s.turnOverDegrees ?? 0).toFixed(0)} deg`,
    limit: hips ? `${SEAM_HIPS_CM} cm` : `${SEAM_TURN_DEGREES} deg`,
    measured: hips ? s.hipsOverCm : s.turnOverDegrees,
    max: hips ? SEAM_HIPS_CM : SEAM_TURN_DEGREES,
    unit: hips ? 'cm' : 'deg',
    status: s.pops ? 'warn' : 'pass',
    note: `${place(s)}${step}${others}${left ? ` ${left} not measured.` : ''}`,
  };
  return [line];
}

/** The seam pop and hold seam lines, each for its worst seam. */
export function seamLines(result: RecipeBakeResult) {
  const seams = result.seams ?? [];
  const pops = seamLine(
    'seam',
    'Seam pop',
    seams.filter(s => s.kind === 'join'),
    s => `${s.about}, frame ${s.frame}. `
  );
  const holds = seamLine(
    'hold-seam',
    'Hold seam',
    seams.filter(s => s.kind !== 'join'),
    s =>
      `The ${s.freeze ? 'freeze' : 'hold'} on ${s.about} ${s.kind === 'hold-in' ? 'eases in' : 'eases out'} at frame ${s.frame}. `
  );
  return { pops, holds };
}

/** A line per two handed grip: how far its hands drift apart. */
export function gapLines(result: RecipeBakeResult): RecipeReviewLine[] {
  return (result.grips ?? []).flatMap(g => {
    if (g.gapCm == null) return [];
    const warn = g.gapCm > GRIP_GAP_LIMIT_CM;
    return [
      {
        id: `grip-gap-${g.layer}`,
        label: 'Grip hand gap',
        about: `both hands (${g.layer})`,
        value: `${g.gapCm.toFixed(1)} cm`,
        limit: `${GRIP_GAP_LIMIT_CM} cm`,
        measured: g.gapCm,
        max: GRIP_GAP_LIMIT_CM,
        unit: 'cm',
        status: warn ? 'warn' : 'pass',
        note:
          warn && g.gapFrame != null
            ? `Both hands on one grip: they move ${g.gapCm.toFixed(1)} cm from where they sit at its first whole frame, most at frame ${g.gapFrame}.`
            : 'Both hands on one grip stay as far apart as at its first whole frame.',
      },
    ];
  });
}

/** The mirror line of a mirrored recipe. */
export function mirrorLines(result: RecipeBakeResult): RecipeReviewLine[] {
  const m = result.mirror;
  if (!m) return [];
  return [
    {
      id: 'mirror',
      label: 'Mirror',
      about: null,
      value: `${m.pairs} pairs swapped`,
      limit: 'every side bone',
      measured: null,
      max: null,
      unit: null,
      status: m.unpaired.length ? 'warn' : 'pass',
      note: m.unpaired.length
        ? `${result.assetName} has no other side for ${m.unpaired.slice(0, 3).join(', ')}, so they stay where they are.`
        : 'Every left and right bone of the model has its other side, so all of them swap.',
    },
  ];
}
