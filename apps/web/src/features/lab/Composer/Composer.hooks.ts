import type { LabView } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { directionId } from '@/state/lab';
import { useLabStore } from '@/state/use-lab-store';

/** The draft brief, whether it adds to an earlier direction, and whether Codex can run it. */
export function useComposer(view: LabView) {
  const draft = useLabStore(s => s.draft);
  const setDraft = useLabStore(s => s.setDraft);
  const generate = useLabStore(s => s.generate);
  const busy = useLabStore(s => s.busy);
  const newDraft = useLabStore(s => s.newDraft);
  const codex = useAppStore(s => s.tools.reports.find(r => r.id === 'codex'));
  const existing = view.directions.find(d => d.id === directionId(draft.name));
  const ready = draft.name.trim() !== '' && draft.prompt.trim() !== '';
  const codexReady = !codex || codex.status === 'found';
  return { draft, setDraft, generate, busy, newDraft, codex, existing, ready, codexReady };
}
