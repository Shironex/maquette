import { PANEL_TITLE } from '@/features/common/Panel';
import { RecheckButton, ToolList, useToolHeadline } from '@/features/toolchain/ToolList';

export default function ToolchainStep() {
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
