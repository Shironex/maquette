import { Button, ToolCheckRow } from '@maquette/ui';
import type { ToolId } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useAppStore } from '@/state/app-store';
import {
  SHORT_NAME,
  TOOL_ORDER,
  rowHint,
  rowStatus,
  rowSummary,
  summarizeTools,
} from '@/state/toolchain';

export interface ToolListProps {
  density: 'roomy' | 'compact';
}

/** Every tool with its detection result and a Locate button. Reads the app store. */
export function ToolList({ density }: ToolListProps) {
  const { reports, checking, busyTool } = useAppStore(s => s.tools);
  const locate = useAppStore(s => s.locateTool);
  const roomy = density === 'roomy';

  const rows = TOOL_ORDER.map(tool => ({ tool, report: reports.find(r => r.id === tool.id) }));
  return (
    <div>
      {rows.map(({ tool, report }) => {
        const busy = checking || busyTool === tool.id || !report;
        const status = report ? rowStatus(report, busy) : 'checking';
        const usable = report?.status === 'found' && !report.warning;
        return (
          <ToolCheckRow
            key={tool.id}
            name={roomy ? tool.label : SHORT_NAME[tool.id]}
            required={tool.required}
            status={status}
            density={density}
            summary={report && !busy ? rowSummary(report, density) : busy ? 'checking…' : undefined}
            hint={report && !busy && roomy ? rowHint(report) : undefined}
            action={
              <LocateButton
                tool={tool.id}
                label={tool.label}
                quiet={usable}
                disabled={busy}
                onLocate={locate}
                fixedWidth={roomy}
              />
            }
          />
        );
      })}
    </div>
  );
}

function LocateButton({
  tool,
  label,
  quiet,
  disabled,
  onLocate,
  fixedWidth,
}: {
  tool: ToolId;
  label: string;
  quiet: boolean;
  disabled: boolean;
  onLocate: (tool: ToolId) => Promise<void>;
  fixedWidth: boolean;
}) {
  return (
    <Button
      size="sm"
      variant={quiet ? 'ghost' : 'secondary'}
      disabled={disabled}
      aria-label={`Locate ${label}`}
      className={fixedWidth ? 'w-[72px]' : undefined}
      onClick={() => void onLocate(tool)}
    >
      Locate…
    </Button>
  );
}

export function RecheckButton() {
  const checking = useAppStore(s => s.tools.checking);
  const detect = useAppStore(s => s.detectTools);
  return (
    <Button size="sm" icon="arrow-clockwise" loading={checking} onClick={() => void detect()}>
      Re-check all
    </Button>
  );
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
