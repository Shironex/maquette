/**
 * The mock review's pose sheet and fixes, as the core picks and words them
 * (bake/sheet.rs, bake/fixes.rs): the frames worth a look (seams, keys,
 * holds, else where each change starts and ends), and for each warning a
 * change that answers it, best first.
 */
import type {
  KeyedLimits,
  RecipeBakeResult,
  RecipeDoc,
  RecipeReviewFix,
  RecipeSheetFrame,
  SeamCheck,
} from '@/ipc/client';
import { keyFrame } from '@/state/recipe-keys';
import { frameOfShare, plain, timeline } from '@/state/recipe-timeline';
import { SEAM_HIPS_CM, SEAM_TURN_DEGREES, hipsWorse, ratioOf } from './recipe-seams';

const SHEET_FRAMES = 8;
const MOST_EASE = 12;
const AIM = 0.8;
const ORDER: Record<RecipeSheetFrame['kind'], number> = { seam: 0, key: 1, hold: 2, change: 3 };

function seamDetail(s: SeamCheck): string {
  if (hipsWorse(s) && (s.hipsOverCm ?? 0) > 0) return `hips jump ${(s.hipsCm ?? 0).toFixed(1)} cm`;
  if ((s.turnOverDegrees ?? 0) > 0)
    return `${s.bone ?? 'a bone'} turns ${(s.turnDegrees ?? 0).toFixed(0)} deg`;
  return s.blend === 0 ? 'cuts straight' : s.blend == null ? 'smooth' : `blends over ${s.blend} f`;
}

/** The frames of `doc`'s bake worth a look, as the core picks them. */
export function sheetOf(
  doc: RecipeDoc,
  seams: SeamCheck[],
  joints: KeyedLimits | null
): RecipeSheetFrame[] {
  const recipe = doc.recipe;
  const t = timeline(recipe);
  const found: RecipeSheetFrame[] = [];
  let n = 0;
  for (const s of seams) {
    if (s.kind === 'join') {
      n += 1;
      found.push({
        frame: s.frame,
        kind: 'seam',
        label: `seam ${n}`,
        detail: seamDetail(s),
        warn: s.pops,
      });
    } else {
      const into = s.kind === 'hold-in';
      found.push({
        frame: s.frame,
        kind: 'hold',
        label: `${s.freeze ? 'freeze' : 'hold'} ${into ? 'starts' : 'ends'}`,
        detail: `${into ? 'eases in' : 'eases out'}, ${(s.turnDegrees ?? 0).toFixed(0)} deg`,
        warn: s.pops,
      });
    }
  }
  for (const layer of recipe.layers.filter(l => l.kind === 'pose'))
    for (const key of layer.keys ?? []) {
      const clip = key.clip ?? 0;
      const at = plain(recipe)
        ? key.frame <= t.frames
          ? key.frame
          : null
        : keyFrame(t, clip, key.frame);
      if (at == null) continue;
      const bones = Object.keys(key.pose);
      const hit = joints?.hits.find(
        h => h.layer === layer.id && h.clip === clip && h.frame === key.frame
      );
      found.push({
        frame: at,
        kind: 'key',
        label: `${bones.length === 1 ? bones[0] : `${bones.length} bones`} key`,
        detail: hit
          ? `${hit.hit.word} ${Math.abs(hit.hit.degrees ?? 0).toFixed(0)} of ${Math.abs(hit.hit.limit ?? 0).toFixed(0)} deg`
          : null,
        warn: Boolean(hit),
      });
    }
  if (!found.length)
    for (const layer of recipe.layers) {
      const { start, end } = layer.settings;
      for (const [share, way] of [
        [start ?? 0, 'starts'],
        [end ?? 1, 'ends'],
      ] as const)
        found.push({
          frame: frameOfShare(t, share),
          kind: 'change',
          label: `${layer.bone} ${way}`,
          detail: null,
          warn: false,
        });
    }
  found.sort(
    (a, b) => Number(b.warn) - Number(a.warn) || ORDER[a.kind] - ORDER[b.kind] || a.frame - b.frame
  );
  const kept: RecipeSheetFrame[] = [];
  for (const f of found)
    if (kept.length < SHEET_FRAMES && !kept.some(k => Math.abs(k.frame - f.frame) <= 1))
      kept.push(f);
  return kept.sort((a, b) => a.frame - b.frame);
}

function framesFor(s: SeamCheck, from: number, most: number) {
  const now = Math.max(from, 1);
  const hips = (s.hipsOverCm ?? 0) * now;
  const turn = (s.turnOverDegrees ?? 0) * now;
  const start = Math.max(from + 2, 2);
  if (start > most) return null;
  let m = most;
  for (let k = start; k <= most; k += 1)
    if (hips / k <= SEAM_HIPS_CM * AIM && turn / k <= SEAM_TURN_DEGREES * AIM) {
      m = k;
      break;
    }
  return { m, hips: hips / m, turn: turn / m };
}

function seamFix(s: SeamCheck): RecipeReviewFix | null {
  const join = s.kind === 'join';
  const from = (join ? s.blend : s.ease) ?? 0;
  const most = join ? s.blendMost : Math.min(s.easeMost ?? MOST_EASE, MOST_EASE);
  if (most == null || (!join && s.at == null)) return null;
  const f = framesFor(s, from, most);
  if (!f) return null;
  const passes = f.hips <= SEAM_HIPS_CM && f.turn <= SEAM_TURN_DEGREES;
  const after = hipsWorse(s) ? `about ${f.hips.toFixed(1)} cm` : `about ${f.turn.toFixed(0)} deg`;
  const moves = hipsWorse(s) ? 'the hip move' : `the ${s.bone ?? 'bone'} turn`;
  const detail = `Spreads ${moves} over more frames.`;
  if (join)
    return {
      id: `blend-${s.clip}`,
      line: 'seam',
      title: `Widen the blend to ${f.m} f`,
      detail,
      after,
      passes,
      edit: { kind: 'blend', clip: s.clip, frames: f.m },
    };
  const into = s.kind === 'hold-in';
  return {
    id: `ease-${s.clip}-${s.at}-${into ? 'in' : 'out'}`,
    line: 'hold-seam',
    title: `Ease the ${s.freeze ? 'freeze' : 'hold'} ${into ? 'in' : 'out'} over ${f.m} f`,
    detail,
    after,
    passes,
    edit: {
      kind: 'hold-ease',
      clip: s.clip,
      at: s.at ?? 0,
      easeIn: into ? f.m : null,
      easeOut: into ? null : f.m,
    },
  };
}

/** The fixes for `result`'s warnings, best first. */
export function fixesOf(result: RecipeBakeResult): RecipeReviewFix[] {
  const found: [number, RecipeReviewFix][] = [];
  for (const s of result.seams ?? []) {
    const fix = s.pops ? seamFix(s) : null;
    if (fix) found.push([ratioOf(s), fix]);
  }
  const worst = result.joints?.hits[0];
  if (worst) {
    const { hit } = worst;
    const limit = Math.abs(hit.limit ?? 0);
    const at =
      worst.bakeFrame == null
        ? `frame ${worst.frame} of clip ${worst.clip + 1}`
        : `frame ${worst.bakeFrame}`;
    found.push([
      Math.abs(hit.degrees ?? 0) / Math.max(limit, 1),
      {
        id: `limit-${worst.layer}-${worst.clip}-${worst.frame}-${hit.bone}`,
        line: 'joints',
        title: `Hold the ${hit.bone} key to its limit`,
        detail: `Sets the ${hit.word} to ${limit.toFixed(0)} deg at ${at}.`,
        after: `${limit.toFixed(0)} deg`,
        passes: true,
        edit: {
          kind: 'key-turn',
          layer: worst.layer,
          clip: worst.clip,
          frame: worst.frame,
          bone: hit.bone,
          axis: hit.axis,
          degrees: hit.limit,
        },
      },
    ]);
  }
  found.sort((a, b) => Number(b[1].passes) - Number(a[1].passes) || b[0] - a[0]);
  return found.map(([, f]) => f);
}
