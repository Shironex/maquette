import type { TimeEdit, TimeKind } from '@/ipc/client';
import { checkDraft } from '@/state/recipe-check';
import {
  TIME_WORDS,
  asKind,
  changeWords,
  lengthChange,
  playsIn,
  speedWords,
  SPEED_PRESETS,
} from '@/state/recipe-time';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipeTimeline } from '@/state/use-recipe-timeline';

const KINDS = (Object.keys(TIME_WORDS) as TimeKind[]).map(k => ({
  value: k,
  label: TIME_WORDS[k],
}));

/** What a hold does, in a line under its fields. */
function holdNote(edit: TimeEdit, name: string): string {
  const at = edit.at ?? 0;
  const frames = edit.frames ?? 0;
  if (edit.kind === 'freeze')
    return `Stops on frame ${at} of ${name} for ${frames} f, and the recipe gets ${frames} f longer.`;
  return `Keeps the pose of frame ${at} of ${name} across the next ${frames} f. The recipe keeps its length.`;
}

/** Time edit `index` of the open recipe: its kind, clip and fields, with what it does to the length. */
export function useTimeForm(index: number) {
  const editing = useRecipeEditorStore(s => s.editing);
  const dt = useRecipeTimeline();
  const draft = editing?.draft ?? null;
  const edit = draft?.time[index] ?? null;
  const clip = edit ? draft?.clips[edit.clip] : undefined;
  if (!draft || !edit || !clip || !dt) return null;
  const store = useRecipeEditorStore.getState();
  const patch = (next: Partial<TimeEdit>) =>
    store.edit(d => ({ ...d, time: d.time.map((t, i) => (i === index ? { ...t, ...next } : t)) }));
  const [a, b] = clip.trim ?? [0, clip.frames];
  return {
    edit,
    clip,
    kinds: KINDS,
    clips: draft.clips.map((c, i) => ({ value: String(i), label: `${i + 1}. ${c.name}` })),
    presets: SPEED_PRESETS.map(s => ({ value: String(s), label: speedWords(s) })),
    preset: SPEED_PRESETS.find(s => s === edit.speed)?.toString(),
    kept: [a, b] as const,
    plays: edit.kind === 'retime' ? playsIn(dt.timeline, edit) : null,
    length: `Recipe ${changeWords(lengthChange(dt.recipe, index))}`,
    note: edit.kind === 'retime' ? null : holdNote(edit, clip.name),
    problem: checkDraft(draft).time[index] ?? null,
    patch,
    setKind: (kind: TimeKind) =>
      store.edit(d => ({
        ...d,
        time: d.time.map((t, i) => (i === index ? asKind(t, kind, d.clips[t.clip] ?? clip) : t)),
      })),
    setClip: (value: string) => patch({ clip: Number(value) }),
    remove: () => {
      store.edit(d => ({ ...d, time: d.time.filter((_, i) => i !== index) }));
      store.pick(null);
    },
  };
}
