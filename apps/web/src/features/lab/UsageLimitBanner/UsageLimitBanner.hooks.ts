import type { LabView } from '@/ipc/client';
import { useNow } from '@/lib/useNow';
import { heldBanner } from '@/state/codex-usage';
import { useCodexUsageStore } from '@/state/use-codex-usage-store';

/** What the banner says about the jobs the usage limit holds, and Limit lifted, retry now. */
export function useUsageLimitBanner(view: LabView) {
  const usage = useCodexUsageStore(s => s.usage);
  const lifted = useCodexUsageStore(s => s.lifted);
  const lifting = useCodexUsageStore(s => s.lifting);
  const lift = useCodexUsageStore(s => s.lift);
  const now = useNow(15_000);
  const banner = heldBanner(view.jobs, usage, now);
  // The answer to the last try, while it still holds.
  const answer = lifted && !lifted.lifted ? lifted.message : null;
  return { banner, answer, lifting, retryNow: () => void lift() };
}
