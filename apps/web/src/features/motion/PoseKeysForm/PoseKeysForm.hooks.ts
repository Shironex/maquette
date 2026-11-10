import { useEffect, useEffectEvent, useMemo } from 'react';
import { checkDraft } from '@/state/recipe-check';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { usePoseKeyEditing, type PoseKeyEditing } from '@/state/use-pose-key-editing';

/** Where a key press goes to its own field instead of a shortcut. */
const typing = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));

/** The pose keys shortcuts: R turn (the gizmo on or off), K key, [ and ] the keys either side, M mirror pose. */
export function usePoseKeyShortcuts(e: PoseKeyEditing) {
  const onKey = useEffectEvent((event: KeyboardEvent) => {
    if (!e.layer || typing(event.target) || event.metaKey || event.ctrlKey || event.altKey) return;
    const act: Record<string, () => void> = {
      r: e.toggleTurning,
      k: e.key,
      '[': () => e.step(-1),
      ']': () => e.step(1),
      m: e.mirror,
    };
    const run = act[event.key.toLowerCase()];
    if (!run) return;
    event.preventDefault();
    run();
  });
  useEffect(() => {
    const listen = (event: KeyboardEvent) => onKey(event);
    window.addEventListener('keydown', listen);
    return () => window.removeEventListener('keydown', listen);
  }, []);
}

/** The pose key change `id`'s form: its keys at the playhead, what is wrong, and removing it. */
export function usePoseKeysForm(id: string) {
  const e = usePoseKeyEditing(id);
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  usePoseKeyShortcuts(e);
  const problem = draft ? (checkDraft(draft).layers[id] ?? null) : null;
  const keys = e.layer?.keys;
  const counts = useMemo(() => {
    const out = new Map<string, number>();
    for (const k of keys ?? [])
      for (const b of Object.keys(k.pose)) out.set(b, (out.get(b) ?? 0) + 1);
    return out;
  }, [keys]);
  return {
    ...e,
    counts,
    problem,
    removeChange: () => {
      const { edit, pick } = useRecipeEditorStore.getState();
      edit(d => ({ ...d, layers: d.layers.filter(l => l.id !== id) }));
      pick(null);
    },
  };
}
