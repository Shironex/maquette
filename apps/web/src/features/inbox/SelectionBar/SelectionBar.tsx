import { Button, IconButton, cn } from '@maquette/ui';
import type { SelectionBarProps } from './SelectionBar.types';

/**
 * The bar while models are ticked: Accept them, Redo them in one batch,
 * Discard them, or Clear the selection. Narrow under the inbox list, one row
 * over the table.
 */
export default function SelectionBar({
  count,
  note,
  acceptable,
  redoBlocker,
  working = false,
  wide = false,
  onAccept,
  onAcceptToLibrary,
  onRedo,
  onDiscard,
  onClear,
}: SelectionBarProps) {
  const none = acceptable === 0 ? 'None of these files can be read' : undefined;
  const accept = (
    <Button
      size="sm"
      variant="primary"
      icon="check"
      disabled={acceptable === 0 || working}
      disabledReason={none}
      loading={working}
      tooltip="Each goes to the asset it was sent for, as its next version. No credits."
      onClick={onAccept}
    >
      Accept {acceptable}
    </Button>
  );
  const redo = (
    <Button
      size="sm"
      icon="arrow-bend-up-right"
      disabled={Boolean(redoBlocker) || working}
      disabledReason={redoBlocker ?? undefined}
      tooltip="One batch, each model prefilled from its own send"
      onClick={onRedo}
    >
      Redo {count}
    </Button>
  );
  const discard = wide ? (
    <Button
      size="sm"
      variant="ghost"
      icon="trash"
      disabled={working}
      tooltip="Moves them aside into inbox/.discarded, not deleted"
      onClick={onDiscard}
    >
      Discard
    </Button>
  ) : (
    <IconButton
      icon="trash"
      size="sm"
      variant="secondary"
      label={`Discard ${count}`}
      hint="Moves them aside into inbox/.discarded, not deleted"
      disabled={working}
      onClick={onDiscard}
    />
  );
  return (
    <div
      role="toolbar"
      aria-label="Selected models"
      className={cn(
        'flex gap-2 border-line bg-raised px-3 py-2.5',
        wide ? 'items-center rounded-card border' : 'flex-col border-t'
      )}
    >
      <div className={cn('flex min-w-0 items-center gap-2', wide && 'flex-1')}>
        <span className="min-w-0 truncate">
          <span className="font-semibold">{count} selected</span>
          {note ? <span className="text-fg-mute"> · {note}</span> : null}
        </span>
        {wide ? null : <span className="flex-1" />}
        {wide ? null : (
          <Button size="sm" variant="ghost" tooltip="Clears the selection" onClick={onClear}>
            Clear
          </Button>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {accept}
        {redo}
        {wide && onAcceptToLibrary ? (
          <Button
            size="sm"
            icon="books"
            disabled={acceptable === 0 || working}
            disabledReason={none}
            tooltip="Accepts them, then adds each asset to the library"
            onClick={onAcceptToLibrary}
          >
            Accept and add to library
          </Button>
        ) : null}
        {discard}
        {wide ? (
          <Button size="sm" variant="ghost" tooltip="Clears the selection" onClick={onClear}>
            Clear
          </Button>
        ) : null}
      </div>
    </div>
  );
}
