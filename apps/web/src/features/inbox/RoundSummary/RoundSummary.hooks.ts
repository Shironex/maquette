import { useEffect, useState } from 'react';
import { commands, type InboxRound } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useTriageStore } from '@/state/use-triage-store';

/** Midnight of today here: a round on the clear Inbox is the day's. */
function startOfToday(): number {
  const now = clock.now();
  return new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
}

/** Today's numbers, read again whenever a model comes or goes. */
export function useRoundSummary() {
  const [round, setRound] = useState<InboxRound | null>(null);
  const items = useInboxStore(s => s.snapshot?.items.length ?? 0);
  const discarded = useTriageStore(s => s.discarded.length);
  const waiting = useProvidersStore(
    s => s.jobs.filter(j => j.provider === 'by-hand' && j.state === 'waiting').length
  );
  useEffect(() => {
    let current = true;
    commands
      .inboxRound(startOfToday())
      .then(next => {
        if (current) setRound(next);
      })
      .catch(() => {
        if (current) setRound(null);
      });
    return () => {
      current = false;
    };
  }, [items, discarded]);
  return { round, items, waiting };
}
