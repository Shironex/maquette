import { Select } from '@maquette/ui';
import type { PlannedAdd } from '@/ipc/client';
import type { ClashChoice } from '@/state/add-many';

export interface ClashRowProps {
  planned: PlannedAdd;
  choice: ClashChoice;
  onChoose: (choice: ClashChoice) => void;
  disabled: boolean;
}

/** One model that meets a library model: what it meets, how alike they look, and the choice. */
export default function ClashRow({ planned, choice, onChoose, disabled }: ClashRowProps) {
  const looks =
    planned.looks === null ? 'no picture to compare' : `looks like ${planned.looks.toFixed(2)}`;
  const meets =
    planned.state === 'name-clash'
      ? `library has ${planned.entryName ?? planned.entry}, ${looks}`
      : `looks like ${planned.entryName ?? planned.entry} (${planned.looks?.toFixed(2) ?? 'no picture'})`;
  const options: { value: ClashChoice; label: string }[] = [
    { value: 'version', label: 'Add as a new version' },
    { value: 'rename', label: planned.state === 'name-clash' ? 'Keep both, rename' : 'Keep both' },
    { value: 'skip', label: 'Skip' },
  ];
  return (
    <li className="flex items-center gap-3 rounded-control border border-line bg-raised px-3 py-2">
      <span className="flex min-w-0 flex-1 flex-col">
        <b className="truncate">{planned.name}</b>
        <span className="truncate text-meta text-fg-mute">{meets}</span>
      </span>
      <Select
        aria-label={`What to do with ${planned.name}`}
        size="sm"
        options={options}
        value={choice}
        disabled={disabled}
        onChange={onChoose}
        fieldClassName="w-[190px] flex-none"
      />
    </li>
  );
}
