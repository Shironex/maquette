import { useState } from 'react';
import type { PackRecord } from '@/ipc/client';
import {
  FILTER_LABEL,
  filterCounts,
  inFilter,
  needsYou,
  nextModel,
  passes,
  readme,
  settled,
  timeLeft,
  type CheckFilter,
} from '@/state/packs';
import { usePackStore } from '@/state/use-pack-store';

const FILTERS: CheckFilter[] = ['all', 'pass', 'look', 'failed', 'running', 'waiting'];

/** The check's filter and what each part of the view shows for `record`. */
export function usePackCheck(record: PackRecord) {
  const store = usePackStore();
  const [filter, setFilter] = useState<CheckFilter>('all');
  const counts = filterCounts(record.items);
  const done = record.items.filter(settled).length;
  const total = record.items.length;
  const passing = record.items.filter(passes).length;
  const warned = record.items.filter(i => i.state === 'warn').length;
  const stopping = store.stopping === record.id && record.state === 'running';
  const during = stopping ? 'stopping after this model' : (timeLeft(record.items) ?? 'starting');
  return {
    busy: store.busy,
    decide: store.decide,
    zip: store.zip,
    stop: store.stop,
    resume: () => store.resume(record.id),
    filter,
    setFilter: (id: string) => setFilter(id as CheckFilter),
    tabs: FILTERS.map(f => ({ id: f, label: FILTER_LABEL[f], count: counts[f] })),
    shown: record.items.filter(i => inFilter(i, filter)),
    needs: needsYou(record.items),
    progress: `${done} of ${total} done${record.state === 'running' ? ` · ${during}` : ''}`,
    percent: total ? (done / total) * 100 : 0,
    passing,
    total,
    zipAll: passing + warned,
    stoppedAt: nextModel(record.items),
    stopping,
    readme: readme(record),
  };
}
