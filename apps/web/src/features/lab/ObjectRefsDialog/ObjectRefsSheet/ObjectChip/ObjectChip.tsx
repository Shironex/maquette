import { Icon, cn } from '@maquette/ui';

export interface ObjectChipProps {
  name: string;
  /** The notes for Codex, when the object has any. */
  notes: string | undefined;
  /** This object's editor is open. */
  editing: boolean;
  onToggle: () => void;
  onRemove: () => void;
}

/** One object in the field: click to edit its name and notes, or remove it. */
export default function ObjectChip({ name, notes, editing, onToggle, onRemove }: ObjectChipProps) {
  return (
    <span
      className={cn(
        'group/chip inline-flex h-[26px] max-w-full items-center gap-1 rounded-pill border bg-clay-soft pr-1 pl-1 text-[12.5px] text-fg',
        editing ? 'border-clay' : 'border-clay-line'
      )}
    >
      <button
        type="button"
        aria-label={`Edit ${name}`}
        aria-expanded={editing}
        title={notes ?? 'Add notes or rename'}
        onClick={e => {
          e.stopPropagation();
          onToggle();
        }}
        className="flex min-w-0 cursor-pointer items-center gap-1 rounded-pill px-1.5"
      >
        <span className="truncate">{name}</span>
        <Icon
          name={notes ? 'file-text' : 'note-pencil'}
          size={12}
          className={cn(
            'flex-none',
            notes
              ? 'text-clay-text'
              : 'text-fg-mute opacity-60 group-hover/chip:opacity-100 group-focus-within/chip:opacity-100'
          )}
        />
      </button>
      <button
        type="button"
        aria-label={`Remove ${name}`}
        onClick={onRemove}
        className="grid size-[18px] cursor-pointer place-items-center rounded-full text-fg-mute hover:bg-panel hover:text-fg"
      >
        <Icon name="x" size={11} />
      </button>
    </span>
  );
}
