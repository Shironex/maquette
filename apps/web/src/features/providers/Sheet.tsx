/**
 * The right-hand sheet the Send to 3D panels open in: a dimmed backdrop,
 * a header with an icon and a close button, Escape and a backdrop click
 * to close. A sheet with unsent changes asks before those close it.
 */
import { useCallback, useRef, useState, type ReactNode } from 'react';
import { Icon, IconButton, type IconName } from '@maquette/ui';
import { useTopEscape } from '@/lib/useTopEscape';
import { ChoiceDialog } from '../common/ChoiceDialog';

export function SectionTitle({ children, aside }: { children: string; aside?: ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <h3 className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
        {children}
      </h3>
      {aside ? <span className="text-meta text-fg-mute">{aside}</span> : null}
    </div>
  );
}

export function Sheet({
  id,
  icon,
  title,
  meta,
  onClose,
  dirty = false,
  width = 520,
  children,
}: {
  /** Prefix for the heading id. */
  id: string;
  icon: IconName;
  title: string;
  meta?: ReactNode;
  onClose: () => void;
  /** Unsent changes: Escape, the backdrop and the close button ask first. */
  dirty?: boolean;
  width?: number;
  children: ReactNode;
}) {
  const sheet = useRef<HTMLDivElement>(null);
  const titleId = `${id}-title`;

  const [asking, setAsking] = useState(false);
  const dismiss = useCallback(() => (dirty ? setAsking(true) : onClose()), [dirty, onClose]);

  useTopEscape(sheet, dismiss);

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
        <header className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name={icon} size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id={titleId} className="truncate text-panel-title">
              {title}
            </h2>
            {meta ? <p className="truncate text-meta text-fg-mute">{meta}</p> : null}
          </div>
          <IconButton data-close icon="x" label="Close" onClick={dismiss} />
        </header>
        {children}
      </div>
      {asking ? (
        <ChoiceDialog
          title="Discard changes?"
          onDismiss={() => setAsking(false)}
          choices={[
            { label: 'Discard', variant: 'danger', onClick: onClose },
            { label: 'Keep editing', variant: 'primary', onClick: () => setAsking(false) },
          ]}
        >
          <p>Your settings for this send have not been sent. Closing drops them.</p>
        </ChoiceDialog>
      ) : null}
    </div>
  );
}
