import { useMemo, useState } from 'react';
import type { RecipeDraft } from '@/state/recipe-draft';
import type { RecipePick } from '@/state/recipe-editor.types';
import { checkDraft } from '@/state/recipe-check';
import { statusOf } from '@/state/recipe-lanes';
import { changeWords, lengthChange, newHold, newRetime, timeTitle } from '@/state/recipe-time';
import { V2_LIMITS, keptFrames } from '@/state/recipe-v2-check';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { useRecipeTimeline } from '@/state/use-recipe-timeline';

type What = NonNullable<RecipePick>['what'];

/** One row of the Base edits list. */
export interface BaseEditItem {
  key: string;
  what: What;
  id: string;
  title: string;
  detail: string;
  problem: string | null;
}

/** "8 and 6 f", "8, 6 and 4 f". */
function listWords(values: readonly number[]): string {
  if (values.length < 2) return `${values[0] ?? 0} f`;
  return `${values.slice(0, -1).join(', ')} and ${values[values.length - 1]} f`;
}

function trimDetail(draft: RecipeDraft): string {
  const trimmed = draft.clips.filter(c => c.trim).length;
  if (!trimmed) return 'Nothing trimmed, every frame plays';
  const kept = draft.clips.reduce((sum, c) => sum + keptFrames(c), 0);
  const total = draft.clips.reduce((sum, c) => sum + c.frames, 0);
  const which =
    trimmed === draft.clips.length
      ? draft.clips.length === 1
        ? 'The clip'
        : 'Each clip'
      : `${trimmed} of ${draft.clips.length} clips`;
  return `${which}, ${kept} of ${total} f kept`;
}

/**
 * The open recipe's base edits as a list (the clips joined, the trims, each
 * time edit, the mirror), the one whose settings show, and adding one.
 */
export function useRecipeBaseEdits() {
  const editing = useRecipeEditorStore(s => s.editing);
  const picked = useRecipeEditorStore(s => s.picked);
  const dt = useRecipeTimeline();
  const [adding, setAdding] = useState(false);
  const draft = editing?.draft ?? null;
  const doc = editing?.doc ?? null;
  const items = useMemo<BaseEditItem[]>(() => {
    if (!draft || !dt) return [];
    const problems = checkDraft(draft);
    const names = draft.clips.map(c => c.name);
    const n = draft.clips.length;
    const blends = draft.clips.slice(1).map(c => c.blend?.frames ?? 0);
    const clipProblem = Object.values(problems.clips)[0] ?? problems.general[0] ?? null;
    const out: BaseEditItem[] = [
      {
        key: 'clips',
        what: 'clips',
        id: '',
        title: n === 1 ? '1 clip' : `${n} clips, joined`,
        detail:
          n === 1
            ? `${names[0]} · ${keptFrames(draft.clips[0]!)} f`
            : `${names.join(', ')} · blends ${listWords(blends)}`,
        problem: clipProblem,
      },
      {
        key: 'trim',
        what: 'trim',
        id: String(
          Math.max(
            draft.clips.findIndex(c => c.trim),
            0
          )
        ),
        title: 'Trim',
        detail: trimDetail(draft),
        problem: null,
      },
    ];
    draft.time.forEach((edit, i) => {
      const name = names[edit.clip] ?? `clip ${edit.clip + 1}`;
      out.push({
        key: `time-${i}`,
        what: 'time',
        id: String(i),
        title: timeTitle(edit),
        detail:
          edit.kind === 'retime'
            ? `${name} ${edit.from ?? 0} to ${edit.to ?? 0}, ${changeWords(lengthChange(dt.recipe, i))}`
            : `${name} frame ${edit.at ?? 0}`,
        problem: problems.time[i] ?? null,
      });
    });
    out.push({
      key: 'mirror',
      what: 'mirror',
      id: '',
      title: 'Mirror',
      detail: draft.mirror ? 'On, left and right swapped' : 'Off',
      problem: null,
    });
    return out;
  }, [draft, dt]);
  const stale = useMemo(() => {
    if (!draft || !doc) return [];
    return draft.clips.filter(
      (_, i) => statusOf(doc.base, doc.joined, draft.kept[i]) === 'hash-differs'
    );
  }, [draft, doc]);
  const { edit, pick } = useRecipeEditorStore.getState();
  const isOpen = (item: BaseEditItem) =>
    picked?.what === item.what && (item.what !== 'time' || picked.id === item.id);
  const timeFull = (draft?.time.length ?? 0) >= V2_LIMITS.timeEdits;
  const rest = Boolean(draft?.clips[0]?.rest);
  const addTime = (make: (d: RecipeDraft) => RecipeDraft['time'][number]) => {
    if (!draft || timeFull) return;
    const index = draft.time.length;
    edit(d => ({ ...d, time: [...d.time, make(d)] }));
    pick({ what: 'time', id: String(index) });
  };
  const clipFor = (d: RecipeDraft) => {
    const at = picked?.what === 'trim' ? Number(picked.id) : 0;
    return d.clips[at] ? at : 0;
  };
  const mixamo = draft?.clips.filter(c => c.mixamo).length ?? 0;
  const count = draft?.clips.length ?? 0;
  return {
    items,
    isOpen,
    toggle: (item: BaseEditItem) => pick(isOpen(item) ? null : { what: item.what, id: item.id }),
    stale: stale.map(c => c.name),
    mixamoNote:
      count > 1 && mixamo > 0
        ? `${mixamo === count ? `All ${count} clips are` : `${mixamo} of ${count} clips ${mixamo === 1 ? 'is' : 'are'}`} Mixamo, so the bake is too: left out of export packs. One Mixamo clip is enough to mark it.`
        : null,
    adding,
    setAdding,
    menu: [
      {
        id: 'clip',
        label: 'Add a clip',
        icon: 'film-strip' as const,
        detail: 'Join one from the animation library',
        disabled: rest || count >= V2_LIMITS.clips,
        hint: rest
          ? 'A recipe on the rest pose has no clips to join'
          : `A recipe joins at most ${V2_LIMITS.clips} clips`,
        onSelect: () => {
          if (editing) useRecipesStore.getState().openPicker(editing.project, true);
        },
      },
      ...(
        [
          ['retime', 'Add a retime', 'Speeds up or slows down a range'],
          ['freeze', 'Add a freeze', 'Stops on one frame and adds its length'],
          ['hold-over', 'Add a hold over', 'Keeps a pose across a range, length unchanged'],
        ] as const
      ).map(([kind, label, detail]) => ({
        id: kind,
        label,
        icon: (kind === 'retime' ? 'gauge' : 'pause') as 'gauge' | 'pause',
        detail,
        disabled: timeFull,
        hint: `A recipe holds at most ${V2_LIMITS.timeEdits} time edits`,
        onSelect: () =>
          addTime(d => {
            const at = clipFor(d);
            const clip = d.clips[at]!;
            return kind === 'retime' ? newRetime(clip, at) : newHold(kind, clip, at);
          }),
      })),
      {
        id: 'mirror',
        label: 'Mirror',
        icon: 'arrows-left-right' as const,
        detail: 'Swaps left and right for a left handed copy',
        onSelect: () => pick({ what: 'mirror', id: '' }),
      },
    ],
  };
}
