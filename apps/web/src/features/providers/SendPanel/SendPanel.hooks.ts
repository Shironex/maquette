import { useProvidersStore } from '@/state/use-providers-store';

/** The panel the store has open, if any, and how to close it. */
export function useSendPanel() {
  const panel = useProvidersStore(s => s.panel);
  const close = useProvidersStore(s => s.closePanel);
  return { panel, close };
}
