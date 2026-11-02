import type { KeyboardEvent } from 'react';
import { IconButton, TextField } from '@maquette/ui';
import type { PlanSummary } from '@/ipc/client';

export interface PlanRowProps {
  plan: PlanSummary;
  edited: string;
  /** The name being typed while this row is renamed, else null. */
  renaming: string | null;
  onOpen: () => void;
  onRename: () => void;
  onType: (name: string) => void;
  onCommit: () => void;
  onCancel: () => void;
  onRemove: () => void;
}

/**
 * One canvas in the table: its name (or the rename field), node count,
 * when it was edited, its file, and Rename and Remove. A file that could
 * not be read shows why and can only be removed.
 */
export default function PlanRow(props: PlanRowProps) {
  const { plan, renaming } = props;
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Enter') props.onCommit();
    if (e.key !== 'Escape') return;
    e.stopPropagation();
    props.onCancel();
  };
  const name =
    renaming !== null ? (
      <TextField
        aria-label={`New name for ${plan.name}`}
        autoFocus
        value={renaming}
        maxLength={120}
        hint="Enter saves, Esc keeps the old name"
        onChange={e => props.onType(e.target.value)}
        onBlur={props.onCommit}
        onKeyDown={onKey}
      />
    ) : plan.problem ? (
      <span className="flex flex-col">
        <span className="truncate text-fg">{plan.name}</span>
        <span className="truncate text-meta text-danger">{plan.problem}</span>
      </span>
    ) : (
      <button
        type="button"
        className="max-w-full truncate font-medium text-fg hover:underline focus-visible:outline-2 focus-visible:outline-focus"
        onClick={props.onOpen}
      >
        {plan.name}
      </button>
    );
  return (
    <tr className="h-11 border-t border-line text-fg-dim">
      <td className="truncate py-1 pl-3">{name}</td>
      <td className="px-2 text-right font-mono">{plan.nodes}</td>
      <td className="truncate px-2">{props.edited}</td>
      <td className="truncate px-2 font-mono text-[11.5px] text-fg-mute">{plan.file}</td>
      <td className="pr-3">
        <span className="flex justify-end gap-1">
          {plan.problem ? null : (
            <IconButton
              icon="note-pencil"
              label={`Rename ${plan.name}`}
              size="sm"
              variant="ghost"
              onClick={props.onRename}
            />
          )}
          <IconButton
            icon="trash"
            label={`Remove ${plan.name}`}
            size="sm"
            variant="ghost"
            onClick={props.onRemove}
          />
        </span>
      </td>
    </tr>
  );
}
