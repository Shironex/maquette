import type { CodexUsage } from '@/ipc/client';
import { useNow } from '@/lib/useNow';
import { usageDetail, usageLabel, usageTone } from '@/state/codex-usage';
import { useCodexUsageStore } from '@/state/use-codex-usage-store';

/** The item's words, tone and reset times, and a read on click. */
export function useCodexUsageItem(usage: CodexUsage) {
  const now = useNow(30_000);
  const read = useCodexUsageStore(s => s.read);
  const reading = useCodexUsageStore(s => s.reading);
  return {
    label: usageLabel(usage),
    detail: usageDetail(usage, now),
    tone: usageTone(usage),
    reading,
    read: () => void read(),
  };
}
