import type { LabView } from '@/ipc/client';
import { useLabStore } from '@/state/use-lab-store';

/** The selected direction, or null before one is picked. */
export function useConceptGrid(view: LabView) {
  const selected = useLabStore(s => s.selected);
  return view.directions.find(d => d.id === selected) ?? null;
}
