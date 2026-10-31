import { Button, Icon } from '@maquette/ui';
import { KEYS } from '@/lib/keys';
import { MAX_BATCH } from '@/state/batch';

export interface SelectionBarProps {
  count: number;
  onSend: () => void;
  /** Opens the batch render sheet on the selection. */
  onRender?: () => void;
  onRemove: () => void;
  onClear: () => void;
}

/** The bar under a selection: how many cards, Send to 3D, Remove and Clear. */
export default function SelectionBar({
  count,
  onSend,
  onRender,
  onRemove,
  onClear,
}: SelectionBarProps) {
  const over = count > MAX_BATCH;
  return (
    <div
      role="toolbar"
      aria-label="Selected assets"
      className="absolute bottom-4 left-1/2 z-20 flex -translate-x-1/2 items-center gap-3 rounded-card border border-line-strong bg-panel py-2 pr-2 pl-3.5 shadow-window"
    >
      <span className="grid size-[22px] place-items-center rounded-[6px] bg-clay text-clay-ink">
        <Icon name="check" size={13} />
      </span>
      <span className="font-semibold whitespace-nowrap">
        {count} {count === 1 ? 'asset' : 'assets'} selected
      </span>
      <span className="text-meta whitespace-nowrap text-fg-mute">
        {over ? `The first ${MAX_BATCH} go in one batch` : 'Shift-click to add a range'}
      </span>
      <Button
        variant="primary"
        icon="cube"
        tooltip="Sends the selected assets to 3D in one batch, under one approval"
        onClick={onSend}
      >
        Send to 3D
      </Button>
      {onRender ? (
        <Button
          variant="ghost"
          icon="image"
          tooltip="Renders the selected models in headless Blender: clay, toon, blueprint, turntable, a contact sheet. No credits"
          onClick={onRender}
        >
          Render...
        </Button>
      ) : null}
      <Button
        variant="ghost"
        icon="trash"
        tooltip="Removes the selected assets from the project into the workspace trash; asks first"
        onClick={onRemove}
      >
        Remove...
      </Button>
      <Button
        variant="ghost"
        tooltip="Clears the selection"
        shortcut={KEYS.escape}
        onClick={onClear}
      >
        Clear
      </Button>
    </div>
  );
}
