import { useProvidersStore } from '@/state/use-providers-store';

/** Picking the send a file is for, and using it, busy while that send imports. */
export function useMatchRow(choice: string | null) {
  const choose = useProvidersStore(s => s.chooseMatch);
  const use = useProvidersStore(s => s.useDownload);
  const busy = useProvidersStore(s => (choice ? s.busy.includes(choice) : false));
  return { choose, use, busy };
}
