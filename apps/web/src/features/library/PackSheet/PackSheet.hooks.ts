import { useEffect } from 'react';
import { FORMAT_LABEL } from '@/state/packs';
import { useAppStore } from '@/state/use-app-store';
import { usePackStore } from '@/state/use-pack-store';

/**
 * Closes a project's pack when another project opens: the switcher's shortcut
 * works under the sheet. A running pack goes on, and Recent packs has it.
 */
export function useClosesWithProject() {
  const current = useAppStore(s => s.workspace?.project?.id);
  const source = usePackStore(s => s.source);
  const open = usePackStore(s => s.open);
  const close = usePackStore(s => s.close);
  const other = open && source?.from === 'project' && source.project !== current;
  useEffect(() => {
    if (other) close();
  }, [other, close]);
}

/** Which part of a pack the sheet shows, and its heading. */
export function usePackSheet() {
  const s = usePackStore();
  const { record, plan, settings } = s;
  if (!record) {
    return {
      ...s,
      phase: 'settings' as const,
      title: 'Export pack',
      meta: plan
        ? `${plan.label}, made ready for another engine or a friend.`
        : 'Reading the models...',
      width: 640,
    };
  }
  const format = FORMAT_LABEL[record.settings.format].split(',')[0];
  const head = `${record.name} · ${format} · ${record.items.length} models`;
  if (record.state === 'done') {
    return {
      ...s,
      phase: 'done' as const,
      title: `${record.name} is ready`,
      meta: head,
      width: 980,
    };
  }
  const word: Record<string, string> = {
    running: 'Checking',
    waiting: 'Needs you',
    zipping: 'Zipping',
    stopped: 'Stopped',
    failed: 'Failed',
  };
  return {
    ...s,
    settings,
    phase: 'check' as const,
    title: 'Export pack',
    meta: `${head} · ${word[record.state] ?? record.state}`,
    width: 1080,
  };
}
