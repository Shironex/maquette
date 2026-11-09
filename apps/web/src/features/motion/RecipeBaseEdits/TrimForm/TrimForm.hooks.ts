import { checkDraft } from '@/state/recipe-check';
import { outsideTrim, patchClip } from '@/state/recipe-clips';
import { timeTitle } from '@/state/recipe-time';
import { keptFrames } from '@/state/recipe-v2-check';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipeTimeline } from '@/state/use-recipe-timeline';

const seconds = (frames: number, fps: number) => (fps > 0 ? frames / fps : 0).toFixed(2);

/** The trim of the clip picked in the Trim lane (the first clip if none): in, out, what it keeps. */
export function useTrimForm() {
  const editing = useRecipeEditorStore(s => s.editing);
  const picked = useRecipeEditorStore(s => s.picked);
  const dt = useRecipeTimeline();
  const draft = editing?.draft ?? null;
  const { edit, pick } = useRecipeEditorStore.getState();
  const wanted = picked?.what === 'trim' ? Number(picked.id) : 0;
  const index = draft?.clips[wanted] ? wanted : 0;
  const clip = draft?.clips[index] ?? null;
  if (!draft || !clip || !dt) return null;
  const [a, b] = clip.trim ?? [0, clip.frames];
  const fps = dt.timeline.fps;
  const outside = outsideTrim(draft.time, index, [a, b]).map(timeTitle);
  const set = (trim: [number, number]) => edit(d => patchClip(d, index, { trim }));
  return {
    index,
    clip,
    a,
    b,
    clips: draft.clips.map((c, i) => ({ value: String(i), label: `${i + 1}. ${c.name}` })),
    keptLine: `Kept ${keptFrames(clip)} of ${clip.frames} f, ${seconds(keptFrames(clip), fps)} s · Recipe ${dt.timeline.frames} f, ${seconds(dt.timeline.frames, fps)} s`,
    outside,
    problem: checkDraft(draft).clips[index] ?? null,
    choose: (value: string) => pick({ what: 'trim', id: value }),
    setIn: (v: number) => set([Math.round(v), b]),
    setOut: (v: number) => set([a, Math.round(v)]),
    keepAll: () => edit(d => patchClip(d, index, { trim: [0, clip.frames] })),
  };
}
