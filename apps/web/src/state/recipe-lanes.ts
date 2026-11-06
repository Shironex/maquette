/**
 * Where a recipe's base edits sit on the bake's frames, for the editor's
 * lanes: each clip's span, the blend windows over the seams, the trims and
 * each time edit, all on one ruler over the timeline's length. Pure
 * helpers over the timeline (recipe-timeline.ts).
 */
import type { BaseMatch, BaseStatus, RecipeBase, TimeEdit } from '@/ipc/client';
import { editSpan, timeTitle } from './recipe-time';
import type { Timeline } from './recipe-timeline';

/** A part of the ruler, bake frames first to last. */
export interface LaneBlock {
  key: string;
  first: number;
  last: number;
  label: string;
}

export const STATUS_WORDS: Record<BaseStatus, string> = {
  found: 'Found',
  'hash-differs': 'Hash differs',
  missing: 'Missing',
  rest: 'Rest pose',
};

/** "6 to 58": the frames a clip keeps. */
export const keptWords = (clip: RecipeBase) => {
  const [a, b] = clip.trim ?? [0, clip.frames];
  return `${a} to ${b}`;
};

/** Where frame `frame` sits along a ruler of `frames` frames, as a CSS length. */
export const at = (frame: number, frames: number) =>
  `${frames > 0 ? (Math.min(Math.max(frame, 0), frames) / frames) * 100 : 0}%`;

/** How wide frames `first` to `last` are along the ruler, as a CSS length. */
export const across = (first: number, last: number, frames: number) =>
  `${frames > 0 ? (Math.max(last - first, 0) / frames) * 100 : 0}%`;

/** Each clip's span with its name and kept frames. */
export function clipBlocks(t: Timeline, clips: readonly RecipeBase[]): LaneBlock[] {
  return t.clips.map(span => {
    const clip = clips[span.clip];
    return {
      key: `clip-${span.clip}`,
      first: span.first,
      last: span.last,
      label: clip ? `${clip.name} ${keptWords(clip)}` : '',
    };
  });
}

/** The blend windows over the seams that blend (a cut has none). */
export const blendBlocks = (t: Timeline): LaneBlock[] =>
  t.seams
    .filter(s => s.frames > 0)
    .map(s => ({
      key: `blend-${s.clip}`,
      first: s.frame,
      last: s.frame + s.frames,
      label: `Blend, ${s.frames} f`,
    }));

/** A mark over each trimmed clip: "Keeps 6 to 58" (the Clips lane names the clip). */
export function trimBlocks(t: Timeline, clips: readonly RecipeBase[]): LaneBlock[] {
  return t.clips.flatMap(span => {
    const clip = clips[span.clip];
    if (!clip?.trim) return [];
    return [
      {
        key: `trim-${span.clip}`,
        first: span.first,
        last: span.last,
        label: `Keeps ${keptWords(clip)}`,
      },
    ];
  });
}

/** Each time edit of the kinds `kinds` on the bake frames it covers, with its index. */
export function timeBlocks(
  t: Timeline,
  time: readonly TimeEdit[],
  kinds: readonly TimeEdit['kind'][]
): (LaneBlock & { index: number })[] {
  return time.flatMap((edit, index) => {
    if (!kinds.includes(edit.kind)) return [];
    const span = editSpan(t, edit);
    if (!span) return [];
    return [{ key: `time-${index}`, index, first: span[0], last: span[1], label: timeTitle(edit) }];
  });
}

/** The ruler's marks: a labelled one every `major` frames and a small one every `minor`. */
export function rulerMarks(frames: number, fps: number) {
  let major = Math.max(Math.round(fps), 1);
  while (frames / major > 10) major *= 2;
  const minor = major / 5;
  const marks: { frame: number; major: boolean }[] = [];
  for (let f = 0; f < frames; f += minor) marks.push({ frame: f, major: f % major === 0 });
  marks.push({ frame: frames, major: true });
  return marks;
}

/** "24 / 147 f · 0.80 / 4.90 s". */
export function frameReadout(frame: number, frames: number, fps: number): string {
  const seconds = (f: number) => (fps > 0 ? f / fps : 0).toFixed(2);
  return `${frame} / ${frames} f · ${seconds(frame)} / ${seconds(frames)} s`;
}

/** How clip `index` of the draft stands in the library: through the file's clip it keeps. */
export function statusOf(
  base: BaseMatch,
  joined: readonly BaseMatch[] | undefined,
  kept: number | null | undefined
): BaseStatus {
  if (kept == null) return 'found';
  const match = kept === 0 ? base : joined?.[kept - 1];
  return match?.status ?? 'missing';
}
