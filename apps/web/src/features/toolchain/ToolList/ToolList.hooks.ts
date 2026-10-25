import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useAppStore } from '@/state/use-app-store';
import { TOOL_ORDER, summarizeTools } from '@/state/toolchain';

/** The rows in detection order, each with its report once the check returns. */
export function useToolList() {
  const { reports, checking, busyTool } = useAppStore(s => s.tools);
  const locate = useAppStore(s => s.locateTool);
  const rows = TOOL_ORDER.map(tool => ({ tool, report: reports.find(r => r.id === tool.id) }));
  return { rows, checking, busyTool, locate };
}

/** "5 of 6 found" and "Checked 4 s ago", for the onboarding panel header. */
export function useToolHeadline() {
  const { reports, checking, checkedAt } = useAppStore(s => s.tools);
  const now = useNow();
  const summary = summarizeTools(reports);
  const headline =
    checking && reports.length === 0
      ? 'Checking your tools'
      : `${summary.found} of ${summary.total || TOOL_ORDER.length} found`;
  const checked = checking ? 'Checking…' : checkedAt ? `Checked ${formatAgo(checkedAt, now)}` : '';
  return { headline, checked, summary };
}
