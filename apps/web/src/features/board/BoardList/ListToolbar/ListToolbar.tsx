import { SegmentedControl, TextField } from '@maquette/ui';
import type { ListToolbarProps } from './ListToolbar.types';
import { useStageLabels } from '@/state/use-stage-labels';
import { stageOptions } from './stageOptions';

/** The list's stage filter and search, with how many rows they leave. */
export default function ListToolbar({ prefs, shown, total, onChange }: ListToolbarProps) {
  const labels = useStageLabels();
  const count =
    shown === total ? `${total} ${total === 1 ? 'asset' : 'assets'}` : `${shown} of ${total}`;
  return (
    <div className="flex flex-none items-center gap-3">
      <SegmentedControl
        label="Stage filter"
        options={stageOptions(labels)}
        value={prefs.stage}
        onChange={stage => onChange({ stage })}
      />
      <TextField
        type="search"
        icon="magnifying-glass"
        aria-label="Search assets"
        placeholder="Search by name or kind"
        value={prefs.query}
        maxLength={120}
        onChange={e => onChange({ query: e.target.value })}
        fieldClassName="w-[240px]"
      />
      <span className="ml-auto font-mono text-[11.5px] text-fg-mute">{count}</span>
    </div>
  );
}
