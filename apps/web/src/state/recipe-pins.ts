/**
 * Pins: a hand or foot that stays put across part of a recipe, held last,
 * after every other change. A step for step copy of the core's rules
 * (`recipes/pins.rs`, cases both agree on in `fixtures/pins.json`): the
 * timeline frames a pin holds on, how it eases in and out, what of a foot
 * stays flat; plus the editor's starting settings, its checks and the rows
 * of the Pin lane. Pure helpers.
 */
import type { LayerSettings, PinFlat, RecipeBlank, RecipeLayer } from '@/ipc/client';
import { boneWords } from './recipe-bones';
import type { Timeline } from './recipe-timeline';

/** The frames a pin eases in and out over unless told. */
export const PIN_EASE_FRAMES = 3;
/** The most frames a pin eases over, as the core caps it. */
export const MAX_PIN_EASE = 100_000;
/** A pinned hand or foot may drift this far from its spot in the bake, cm. */
export const PIN_DRIFT_LIMIT_CM = 1;
const EPS = 1e-6;

/** One pin on the timeline's frames, as the bake and the preview hold it. */
export interface PinSpan {
  layer: string;
  bone: string;
  first: number;
  last: number;
  easeIn: number;
  easeOut: number;
  weight: number;
  /** A foot's: what stays flat. Undefined for a hand, which keeps its turn. */
  flat?: PinFlat;
  /** The blank it stays on; undefined stays where the bone is at `first`. */
  target?: string;
}

export const FLAT_WORDS: Record<PinFlat, string> = {
  'toe-and-heel': 'Toe and heel',
  heel: 'Heel',
  off: 'Off',
};

export const isFoot = (bone: string) => bone === 'leftFoot' || bone === 'rightFoot';

const n = (v: number | null | undefined, or: number) =>
  typeof v === 'number' && Number.isFinite(v) ? v : or;

/** What of a pinned foot stays flat: its own, else toe and heel. Undefined for a hand. */
export const flatOf = (layer: Pick<RecipeLayer, 'bone' | 'settings'>): PinFlat | undefined =>
  isFoot(layer.bone) ? (layer.settings.flat ?? 'toe-and-heel') : undefined;

/** Where `layer`, a pin, holds on timeline `t`; null when no frame falls within its range. */
export function spanOn(t: Timeline, layer: RecipeLayer): PinSpan | null {
  const s = layer.settings;
  const start = n(s.start, 0);
  const end = n(s.end, 1);
  const first = t.keys.findIndex(k => k.share >= start - EPS);
  let last = -1;
  for (let i = t.keys.length - 1; i >= 0; i -= 1) {
    if ((t.keys[i]?.share ?? Infinity) <= end + EPS) {
      last = i;
      break;
    }
  }
  if (first < 0 || last < first) return null;
  const flat = flatOf(layer);
  return {
    layer: layer.id,
    bone: layer.bone,
    first,
    last,
    easeIn: s.easeIn ?? PIN_EASE_FRAMES,
    easeOut: s.easeOut ?? PIN_EASE_FRAMES,
    weight: Math.min(Math.max(n(s.weight, 1), 0), 1),
    ...(flat ? { flat } : {}),
    ...(s.target ? { target: s.target } : {}),
  };
}

/** Every pin of `layers` on timeline `t`, in the order they are laid. */
export const pinSpans = (t: Timeline, layers: readonly RecipeLayer[]): PinSpan[] =>
  layers.flatMap(l => (l.kind === 'pin' ? (spanOn(t, l) ?? []) : []));

const smooth = (x: number) => {
  const c = Math.min(Math.max(x, 0), 1);
  return c * c * (3 - 2 * c);
};

/** How much of `pin` holds at timeline frame `frame`, 0 to 1, weight in. */
export function pinAmount(pin: PinSpan, frame: number): number {
  if (frame < pin.first - EPS || frame > pin.last + EPS) return 0;
  const up = pin.easeIn > 0 ? smooth((frame - pin.first) / pin.easeIn) : 1;
  const down = pin.easeOut > 0 ? smooth((pin.last - frame) / pin.easeOut) : 1;
  return pin.weight * Math.min(up, down);
}

/** The frames where `pin` holds whole, which the drift check measures; null when none. */
export function heldFrames(pin: PinSpan): [number, number] | null {
  const from = pin.first + pin.easeIn;
  const to = Math.max(pin.last - pin.easeOut, 0);
  return from <= to ? [from, to] : null;
}

/** A new pin's settings, or a change switched to a pin: eases and (on a foot) flat filled in. */
export function pinSettings(settings: LayerSettings, bone: string): LayerSettings {
  return {
    ...settings,
    easeIn: settings.easeIn ?? PIN_EASE_FRAMES,
    easeOut: settings.easeOut ?? PIN_EASE_FRAMES,
    flat: isFoot(bone) ? (settings.flat ?? 'toe-and-heel') : null,
  };
}

/** `layer`, a pin, moved to `bone`: a foot keeps flat (toe and heel to start), a hand drops it. */
export const withPinBone = (layer: RecipeLayer, bone: string): RecipeLayer => ({
  ...layer,
  bone,
  settings: {
    ...layer.settings,
    flat: isFoot(bone) ? (layer.settings.flat ?? 'toe-and-heel') : null,
  },
});

/** What the core would refuse in a pin's own settings, or null. */
export function pinProblem(layer: RecipeLayer, blanks: readonly RecipeBlank[]): string | null {
  const s = layer.settings;
  if (s.target && !blanks.some(b => b.id === s.target))
    return `It stays on the blank ${s.target}, which the recipe does not have.`;
  if (s.flat && !isFoot(layer.bone)) return 'Only a foot keeps flat. Set keep flat on a foot.';
  for (const ease of [s.easeIn, s.easeOut]) {
    if (ease == null) continue;
    if (!Number.isInteger(ease) || ease < 0 || ease > MAX_PIN_EASE)
      return `Eases are whole frames, 0 to ${MAX_PIN_EASE}.`;
  }
  return null;
}

/** "Where it is at frame 12", or the blank it stays on. */
export function staysAtWords(
  layer: RecipeLayer,
  firstFrame: number,
  blanks: readonly RecipeBlank[]
) {
  const target = layer.settings.target;
  if (!target) return `Where it is at frame ${firstFrame}`;
  const blank = blanks.find(b => b.id === target);
  return blank ? `${blank.id}: ${blank.label}` : target;
}

/**
 * The pins of `layers` stacked into as few rows of the Pin lane as hold them
 * without two overlapping on one row, in order; `frameOf` turns a share into
 * a frame.
 */
export function pinRows(
  layers: readonly RecipeLayer[],
  frameOf: (share: number) => number
): RecipeLayer[][] {
  const ends: number[] = [];
  const rows: RecipeLayer[][] = [];
  for (const layer of layers) {
    if (layer.kind !== 'pin') continue;
    const first = frameOf(n(layer.settings.start, 0));
    const last = frameOf(n(layer.settings.end, 1));
    const row = ends.findIndex(end => end < first);
    if (row < 0) {
      ends.push(last);
      rows.push([layer]);
    } else {
      ends[row] = last;
      rows[row]?.push(layer);
    }
  }
  return rows;
}

/** What the preview measures of one pin over its frames (features/viewer/recipe-pins.ts). */
export interface PinCheck {
  layer: string;
  /** The most the hips move from where they are at the pin's first frame, metres. */
  hips: number;
  /** The limb's length, upper and lower bone, metres. */
  limb: number;
  /** The first frame the spot is out of the limb's reach, or null when it always reaches. */
  outFrame: number | null;
}

const cm = (metres: number) => Math.round(metres * 100);

/**
 * "She can stay on it" and why, or why not: the hips' travel against the
 * limb's reach, as the pin's settings show it.
 */
export function pinCheckWords(check: PinCheck, bone: string): { title: string; detail: string } {
  const foot = isFoot(bone);
  const limb = foot ? 'leg' : 'arm';
  const joint = foot ? 'knee' : 'elbow';
  const what = foot ? 'foot' : 'hand';
  const moves = `The hips move ${cm(check.hips)} cm while it holds.`;
  if (check.outFrame == null)
    return {
      title: `The ${what} can stay on its spot`,
      detail: `${moves} The ${limb} reaches ${cm(check.limb)} cm, so the ${joint} bends and the ${what} holds.`,
    };
  return {
    title: `The ${what} cannot stay on its spot all the way`,
    detail: `${moves} At frame ${check.outFrame} the spot is past the ${limb}'s ${cm(check.limb)} cm, so the ${what} leaves it. Shorten the pin or move its spot.`,
  };
}

/** "Left foot", the bar's words on the Pin lane. */
export const pinLabel = (layer: RecipeLayer) => boneWords(layer.bone);
