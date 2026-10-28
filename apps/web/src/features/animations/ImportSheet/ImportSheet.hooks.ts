import { importTotals } from '@/state/animations';
import { useAnimationsStore } from '@/state/use-animations-store';

const clips = (n: number) => `${n} ${n === 1 ? 'clip' : 'clips'}`;

/** The open import: its plan, what goes in with the person's choices, and the actions. */
export function useImportSheet() {
  const sheet = useAnimationsStore(s => s.importing);
  const busy = useAnimationsStore(s => s.busy);
  const dir = useAnimationsStore(s => s.view?.dir ?? null);
  const { closeImport, pickFiles, changeImport, runImport } = useAnimationsStore.getState();
  const plan = sheet?.plan ?? null;
  const totals = plan && sheet ? importTotals(plan, sheet.choices) : null;
  const add = totals?.add ?? 0;
  return {
    sheet,
    plan,
    busy,
    dir,
    totals,
    add,
    addLine: clips(add),
    importLabel: add ? `Import ${clips(add)}` : 'Import',
    zips: plan?.files.filter(f => f.archive) ?? [],
    singles: plan?.files.some(f => !f.archive) ?? false,
    close: closeImport,
    pickFiles: () => void pickFiles(),
    change: changeImport,
    run: () => void runImport(),
  };
}
