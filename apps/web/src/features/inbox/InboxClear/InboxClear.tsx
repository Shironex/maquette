import { Icon, cn } from '@maquette/ui';
import { DiscardedList } from '../DiscardedList';
import { DownloadsOnHold } from '../DownloadsOnHold';
import { RoundSummary } from '../RoundSummary';
import { WaitingSends } from '../WaitingSends';

export interface InboxClearProps {
  /** The inbox folder, once known. */
  dir?: string;
  watching?: boolean;
}

/**
 * The empty inbox after a round: where new models come from, what the round
 * came to, the By hand sends still waiting, the models discarded lately with
 * Restore and the downloads on hold.
 */
export default function InboxClear({ dir, watching = false }: InboxClearProps) {
  return (
    <div className="flex min-h-0 flex-1 overflow-y-auto px-6 py-10">
      {/* Auto margins centre it while it fits, and let it scroll from the top when it does not. */}
      <div className="m-auto flex w-full max-w-[640px] flex-col gap-6">
        <div className="flex flex-col items-center gap-2 rounded-card border border-line bg-panel px-8 py-10 text-center">
          <span className="grid size-11 place-items-center rounded-full bg-sage-soft text-sage">
            <Icon name="check-circle" size={22} />
          </span>
          <h2 className="text-[18px] leading-6 font-semibold">Inbox clear</h2>
          <p className="max-w-[440px] text-fg-dim">
            Every model has a place. New downloads and sends land here; drop a .glb into{' '}
            {dir ?? 'the inbox folder'} and I check it before it reaches Blender.
          </p>
          <span className="inline-flex items-center gap-2 text-meta text-fg-mute">
            <span className={cn('size-2 rounded-full', watching ? 'bg-sage' : 'bg-fg-mute')} />
            {watching ? 'Watching the folder' : 'The folder is not watched'}
          </span>
        </div>
        <RoundSummary />
        <WaitingSends />
        <DiscardedList limit={8} />
        <DownloadsOnHold />
      </div>
    </div>
  );
}
