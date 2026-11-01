import { ToolCheckRow } from '@maquette/ui';
import { SHORT_NAME, rowHint, rowStatus, rowSummary } from '@/state/toolchain';
import { LocateButton } from './LocateButton';
import { useToolList } from './ToolList.hooks';

export interface ToolListProps {
  density: 'roomy' | 'compact';
}

/** Every tool with its detection result and a Locate button. Reads the app store. */
export default function ToolList({ density }: ToolListProps) {
  const { rows, checking, busyTool, locate } = useToolList();
  const roomy = density === 'roomy';

  const items = rows.map(({ tool, report }) => {
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
        summary={report && !busy ? rowSummary(report, density) : busy ? 'checking...' : undefined}
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
  });
  return <div>{items}</div>;
}
