import { useAppStore } from '@/state/use-app-store';
import { summarizeTools } from '@/state/toolchain';

/** The tool summary once a check has finished, else null. */
export function useToolchainFootNote() {
  const { reports, checking } = useAppStore(s => s.tools);
  if (checking || reports.length === 0) return null;
  return summarizeTools(reports);
}
