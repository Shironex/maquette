/**
 * The right-hand sheet the Send to 3D panels open in: a dimmed backdrop,
 * a header with an icon and a close button, Escape and a backdrop click
 * to close. A sheet with unsent changes asks before those close it.
 */
import { Icon, IconButton } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useSheetDismiss } from './Sheet.hooks';
import type { SheetProps } from './Sheet.types';
import { KEYS } from '@/lib/keys';

export default function Sheet({
  id,
  icon,
  title,
  meta,
  onClose,
  dirty = false,
  width = 520,
  children,
}: SheetProps) {
  const { sheet, asking, dismiss, keepEditing } = useSheetDismiss(dirty, onClose);
  const titleId = `${id}-title`;

  return (
    <div
      className="fixed inset-0 z-40 flex justify-end bg-[rgba(16,15,14,0.55)]"
      onClick={e => {
        if (e.target === e.currentTarget) dismiss();
      }}
    >
      <div
        ref={sheet}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        style={{ width }}
        className="flex h-full max-w-full flex-col border-l border-line-strong bg-panel shadow-window"
      >
        <div className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name={icon} size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id={titleId} className="truncate text-panel-title">
              {title}
            </h2>
            {meta ? <p className="truncate text-meta text-fg-mute">{meta}</p> : null}
          </div>
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={dismiss} />
        </div>
        {children}
      </div>
      {asking ? (
        <ChoiceDialog
          title="Discard changes?"
          onDismiss={keepEditing}
          choices={[
            { label: 'Discard', variant: 'danger', onClick: onClose },
            { label: 'Keep editing', variant: 'primary', onClick: keepEditing },
          ]}
        >
          <p>Your settings for this send have not been sent. Closing drops them.</p>
        </ChoiceDialog>
      ) : null}
    </div>
  );
}
