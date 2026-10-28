import { Button, Icon, IconButton } from '@maquette/ui';
import { homePath } from '@/lib/format';
import { KEYS } from '@/lib/keys';
import { SectionTitle } from '../../SectionTitle';
import { ArrivalRow } from './ArrivalRow';
import { LookCard } from './LookCard';
import { usePickerDialog } from './PickerDialog.hooks';

export interface PickerDialogProps {
  onClose: () => void;
}

const LIST = 'm-0 flex list-none flex-col rounded-card border border-line bg-raised p-0';

export default function PickerDialog({ onClose }: PickerDialogProps) {
  const { looking, asking, count, folder, home, ref, now } = usePickerDialog(onClose);
  const looks = looking.map(arrival => <LookCard key={arrival.id} arrival={arrival} now={now} />);
  const rows = asking.map(arrival => <ArrivalRow key={arrival.id} arrival={arrival} now={now} />);
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
        className="flex max-h-full w-[640px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
      >
        <div className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="question" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="arrival-title" className="truncate text-panel-title">
              New downloads
            </h2>
            <p className="truncate text-meta text-fg-mute">
              {folder ? `New in ${homePath(folder, home)}` : 'New downloads'} · {count}{' '}
              {count === 1 ? 'file' : 'files'}
            </p>
          </div>
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={onClose} />
        </div>
        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4">
          {count === 0 ? (
            <p className="py-6 text-center text-meta text-fg-mute">
              Every new download has found its asset.
            </p>
          ) : null}
          {looks.length > 0 ? (
            <section aria-label="Needs a look" className="flex flex-col gap-2">
              <SectionTitle aside={looks.length}>Needs a look</SectionTitle>
              <p className="m-0 text-meta text-fg-mute">
                The name is only close to a send&apos;s, so nothing went in yet.
              </p>
              <ul className={LIST}>{looks}</ul>
            </section>
          ) : null}
          {rows.length > 0 ? (
            <section aria-label="Which asset is this?" className="flex flex-col gap-2">
              <SectionTitle aside={rows.length}>Which asset is this?</SectionTitle>
              <ul className={LIST}>{rows}</ul>
            </section>
          ) : null}
        </div>
        <div className="flex items-center gap-2 border-t border-line px-5 py-3">
          <p className="min-w-0 flex-1 text-meta text-fg-mute">
            Attaching sends the model to the Inbox for that asset. The original stays where it is.
          </p>
          <Button variant="primary" onClick={onClose}>
            Done
          </Button>
        </div>
      </div>
    </div>
  );
}
