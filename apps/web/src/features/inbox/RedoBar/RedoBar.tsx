import { Button } from '@maquette/ui';

export interface RedoBarProps {
  count: number;
  /** Why the ticked models cannot be redone together, or null. */
  blocker: string | null;
  onRedo: () => void;
  onClear: () => void;
}

/** The bar under the inbox list while models are ticked: Redo them in one batch, or Clear. */
export default function RedoBar({ count, blocker, onRedo, onClear }: RedoBarProps) {
  return (
    <div
      role="toolbar"
      aria-label="Selected models"
      className="flex items-center gap-2 border-t border-line bg-raised px-3 py-2.5"
    >
      <span className="min-w-0 flex-1 truncate font-semibold">
        {count} {count === 1 ? 'model' : 'models'} selected
      </span>
      <Button
        size="sm"
        variant="primary"
        icon="arrow-bend-up-right"
        disabled={Boolean(blocker)}
        disabledReason={blocker ?? undefined}
        tooltip="One batch, each model prefilled from its own send"
        onClick={onRedo}
      >
        Redo {count}
      </Button>
      <Button size="sm" variant="ghost" tooltip="Clears the selection" onClick={onClear}>
        Clear
      </Button>
    </div>
  );
}
