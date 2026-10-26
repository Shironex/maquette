import type { ReactNode } from 'react';
import { Icon, IconButton } from '@maquette/ui';
import { KEYS } from '@/lib/keys';
import { useLibraryDialogFrame } from './LibraryDialogFrame.hooks';

export interface LibraryDialogFrameProps {
  id: string;
  title: string;
  help: string;
  onClose: () => void;
  onSubmit: () => void;
  children: ReactNode;
  footer: ReactNode;
}

/** The library dialogs' shell: a books mark, title and help, a body and the buttons. */
export default function LibraryDialogFrame({
  id,
  title,
  help,
  onClose,
  onSubmit,
  children,
  footer,
}: LibraryDialogFrameProps) {
  const { ref, submit } = useLibraryDialogFrame(onClose, onSubmit);
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <form
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={id}
        className="flex w-[520px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
        onSubmit={submit}
      >
        <header className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="books" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id={id} className="truncate text-panel-title">
              {title}
            </h2>
            <p className="text-meta text-fg-mute">{help}</p>
          </div>
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={onClose} />
        </header>
        <div className="flex flex-col gap-3.5 px-5 py-4">{children}</div>
        <footer className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          {footer}
        </footer>
      </form>
    </div>
  );
}
