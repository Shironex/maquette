/**
 * The Timeline view's page wide lanes: the scene's render style and the hero
 * text. Each is one track for the page, keyed at camera stops like an
 * energize track: the site fades the 3D layer into the look and the marked
 * copy in and out as it scrolls. Pure data in, data out.
 */
import type { Easing, LayoutStop, PathKey, TrackLook } from '@/ipc/client';
import type { IconName } from '@maquette/ui';
import { keyMarks, type TimelineDraft } from './timeline';

export type PageLaneId = 'style' | 'text';

export interface PageLane {
  id: PageLaneId;
  name: string;
  icon: IconName;
  hint: string;
}

export const PAGE_LANES: readonly PageLane[] = [
  {
    id: 'style',
    name: 'Render style',
    icon: 'palette',
    hint: 'Fades the 3D scene into a look as the page scrolls',
  },
  {
    id: 'text',
    name: 'Hero text',
    icon: 'file-text',
    hint: 'Fades the hero copy the site marks as the page scrolls',
  },
];

/** The looks the site can draw live; the rest of the render styles only Blender renders. */
export const LOOKS: readonly { value: TrackLook; label: string }[] = [
  { value: 'blueprint', label: 'Blueprint' },
  { value: 'toon', label: 'Toon' },
  { value: 'clay', label: 'Clay' },
];

export const lookName = (look: TrackLook) => LOOKS.find(l => l.value === look)?.label ?? look;

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));

/**
 * A new track, or the same track keyed again from the first stop to the last:
 * the look comes in over the page; the hero text starts shown and fades out.
 */
export function addPageTrack(
  draft: TimelineDraft,
  lane: PageLaneId,
  stops: readonly LayoutStop[]
): TimelineDraft {
  const first = stops[0];
  const last = stops.at(-1);
  const [from, to] = lane === 'style' ? [0, 1] : [1, 0];
  const keys: PathKey[] = [];
  if (first) keys.push({ stop: first.id, amount: from });
  if (last && last !== first) keys.push({ stop: last.id, amount: to });
  const timeline = draft.timeline;
  if (lane === 'style') {
    const style = {
      look: timeline.style?.look ?? ('blueprint' as const),
      easing: timeline.style?.easing ?? ('linear' as const),
      keys,
    };
    return { ...draft, timeline: { ...timeline, style } };
  }
  return {
    ...draft,
    timeline: {
      ...timeline,
      text: { easing: timeline.text?.easing ?? 'linear', keys },
    },
  };
}

export function removePageTrack(draft: TimelineDraft, lane: PageLaneId): TimelineDraft {
  return { ...draft, timeline: { ...draft.timeline, [lane]: null } };
}

/** Sets the amount at a stop, adding the key; null removes it. */
export function setPageKey(
  draft: TimelineDraft,
  lane: PageLaneId,
  stop: string,
  amount: number | null
): TimelineDraft {
  const track = draft.timeline[lane];
  if (!track) return draft;
  const rest = track.keys.filter(k => k.stop !== stop);
  const keys = amount === null ? rest : [...rest, { stop, amount: clamp01(amount) }];
  return { ...draft, timeline: { ...draft.timeline, [lane]: { ...track, keys } } };
}

export function setPageEasing(
  draft: TimelineDraft,
  lane: PageLaneId,
  easing: Easing
): TimelineDraft {
  const track = draft.timeline[lane];
  return track
    ? { ...draft, timeline: { ...draft.timeline, [lane]: { ...track, easing } } }
    : draft;
}

export function setLook(draft: TimelineDraft, look: TrackLook): TimelineDraft {
  const style = draft.timeline.style;
  return style ? { ...draft, timeline: { ...draft.timeline, style: { ...style, look } } } : draft;
}

/** The up and down words on a lane's blocks: "Blueprint in", "Fades out". */
export function pageMarks(
  lane: PageLaneId,
  track: { keys: PathKey[]; look?: TrackLook },
  stops: readonly LayoutStop[]
) {
  if (lane === 'text') return keyMarks(track.keys, stops, 'Fades in', 'Fades out');
  const name = lookName(track.look ?? 'blueprint');
  return keyMarks(track.keys, stops, `${name} in`, `${name} out`);
}

/** "Shown", "Hidden", "Blueprint" or "40% Blueprint", for a key. */
export function pageAmountLabel(lane: PageLaneId, amount: number, look?: TrackLook): string {
  const percent = Math.round(amount * 100);
  if (lane === 'text') return amount >= 1 ? 'shown' : amount <= 0 ? 'hidden' : `${percent}% shown`;
  const name = lookName(look ?? 'blueprint').toLowerCase();
  return amount >= 1 ? name : amount <= 0 ? 'as built' : `${percent}% ${name}`;
}
