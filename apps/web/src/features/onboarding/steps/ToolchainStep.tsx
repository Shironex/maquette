import { Icon } from '@maquette/ui';
import { PANEL_TITLE } from '@/features/common/Panel';
import { RecheckButton, ToolList, useToolHeadline } from '@/features/toolchain/ToolList';
import { useAppStore } from '@/state/app-store';
import { summarizeTools } from '@/state/toolchain';

export function ToolchainStep() {
  const { headline, checked } = useToolHeadline();
  return (
    <section
      aria-label="Toolchain"
      className="mt-8 overflow-hidden rounded-card border border-line bg-panel"
    >
      <div className="flex items-center gap-2.5 border-b border-line px-4 py-3">
        <span className={`${PANEL_TITLE} flex-1`} aria-live="polite">
          {headline}
        </span>
        <span className="text-meta text-fg-mute">{checked}</span>
        <RecheckButton />
      </div>
      <ToolList density="roomy" />
    </section>
  );
}

/** "1 warning. Continue works with warnings. Only Blender is required." */
export function ToolchainFootNote() {
  const { reports, checking } = useAppStore(s => s.tools);
  if (checking || reports.length === 0) return null;
  const summary = summarizeTools(reports);
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
  return (
    <span className="ml-2 inline-flex items-center gap-1.5 text-meta text-fg-mute">
      <span className="inline-flex items-center gap-1.5 text-amber">
        <Icon name="warning" size={14} />
        {summary.warnings === 1 ? '1 warning.' : `${summary.warnings} warnings.`}
      </span>
      Continue works with warnings. Only Blender is required.
    </span>
  );
}
