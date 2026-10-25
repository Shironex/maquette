import { Button, Icon, IconButton } from '@maquette/ui';
import { homePath } from '@/lib/format';
import { KEYS } from '@/lib/keys';
import { ArrivalRow } from './ArrivalRow';
import { usePickerDialog } from './PickerDialog.hooks';

export interface PickerDialogProps {
  onClose: () => void;
}

export default function PickerDialog({ onClose }: PickerDialogProps) {
  const { asking, options, folder, home, ref, now } = usePickerDialog(onClose);
  const rows = asking.map(arrival => (
    <ArrivalRow key={arrival.id} arrival={arrival} options={options} now={now} />
  ));
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="arrival-title"
        className="flex max-h-full w-[620px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
      >
        <header className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="question" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="arrival-title" className="truncate text-panel-title">
              Which asset is this?
            </h2>
            <p className="truncate text-meta text-fg-mute">
              {folder ? `New in ${homePath(folder, home)}` : 'New downloads'} · {asking.length}{' '}
              {asking.length === 1 ? 'file' : 'files'}
            </p>
          </div>
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={onClose} />
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
          {asking.length === 0 ? (
            <p className="py-6 text-center text-meta text-fg-mute">
              Every new download has found its asset.
            </p>
          ) : options.length === 0 ? (
            <p className="py-6 text-center text-meta text-fg-mute">
              No By hand send is waiting for a model any more.
            </p>
          ) : (
            <ul className="m-0 flex list-none flex-col rounded-card border border-line bg-raised p-0">
              {rows}
            </ul>
          )}
        </div>
        <footer className="flex items-center gap-2 border-t border-line px-5 py-3">
          <p className="min-w-0 flex-1 text-meta text-fg-mute">
            I copy each file you add into the inbox. The original stays where it is.
          </p>
          <Button variant="primary" onClick={onClose}>
            Done
          </Button>
        </footer>
      </div>
    </div>
  );
}
