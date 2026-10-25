import { Icon } from '@maquette/ui';
import { useToolchainFootNote } from './ToolchainFootNote.hooks';

/** "1 warning. Continue works with warnings. Only Blender is required." */
export default function ToolchainFootNote() {
  const summary = useToolchainFootNote();
  if (!summary) return null;
  if (summary.blocked) {
    return (
      <span className="ml-2 inline-flex items-center gap-1.5 text-meta text-danger">
        <Icon name="warning-circle" size={14} />
        Blender is required. Install it or locate it to continue.
      </span>
    );
  }
  if (summary.warnings === 0) {
    return (
      <span className="ml-2 inline-flex items-center gap-1.5 text-meta text-sage">
        <Icon name="check-circle" size={14} />
        Everything is ready.
      </span>
    );
  }
  const warnings = summary.warnings === 1 ? '1 warning.' : `${summary.warnings} warnings.`;
  return (
    <span className="ml-2 inline-flex items-center gap-1.5 text-meta text-fg-mute">
      <span className="inline-flex items-center gap-1.5 text-amber">
        <Icon name="warning" size={14} />
        {warnings}
      </span>
      Continue works with warnings. Only Blender is required.
    </span>
  );
}
