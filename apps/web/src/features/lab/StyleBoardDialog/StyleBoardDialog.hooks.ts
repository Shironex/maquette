import { useLabStore } from '@/state/use-lab-store';

/** The draft the dialog is open on and the Lab view it belongs to, or nulls while closed. */
export function useStyleBoardDialog() {
  const style = useLabStore(s => s.style);
  const view = useLabStore(s => s.view);
  return { style, view };
}
