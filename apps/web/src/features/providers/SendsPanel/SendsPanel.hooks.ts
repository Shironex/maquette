import { useCallback } from 'react';
import { waitingByHand } from '@/state/batch';
import { sendCounts, sendsShowTabs, type SendsTab } from '@/state/sends';
import { useProvidersStore } from '@/state/use-providers-store';

/** The panel's open list, its tabs and its line under the title. */
export function useSendsPanel() {
  const queueOpen = useProvidersStore(s => s.queueOpen);
  const checklistOpen = useProvidersStore(s => s.checklistOpen);
  const setQueueOpen = useProvidersStore(s => s.setQueueOpen);
  const setChecklistOpen = useProvidersStore(s => s.setChecklistOpen);
  const jobs = useProvidersStore(s => s.jobs);
  // Stable, or the sheet would take focus back to its close button on every update.
  const close = useCallback(() => {
    setQueueOpen(false);
    setChecklistOpen(false);
  }, [setQueueOpen, setChecklistOpen]);
  const tab: SendsTab = checklistOpen ? 'hand' : 'queue';
  const { inFlight } = sendCounts(jobs);
  const waiting = waitingByHand(jobs).length;
  const handMeta = waiting
    ? `${waiting} ${waiting === 1 ? 'model' : 'models'} to make · no API credits`
    : 'Every model is back';
  return {
    open: queueOpen || checklistOpen,
    tab,
    showTabs: sendsShowTabs(jobs, tab),
    inFlight,
    waiting,
    meta:
      tab === 'hand'
        ? handMeta
        : inFlight
          ? `${inFlight} ${inFlight === 1 ? 'send' : 'sends'} in flight`
          : 'Nothing in flight',
    setTab: (next: SendsTab) => (next === 'hand' ? setChecklistOpen(true) : setQueueOpen(true)),
    close,
  };
}
