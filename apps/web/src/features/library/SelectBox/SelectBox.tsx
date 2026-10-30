import { Icon, Tooltip, cn } from '@maquette/ui';

export interface SelectBoxProps {
  /** The model's name, for the box's label. */
  name: string;
  checked: boolean;
  /** Keeps the box on show while a selection is under way. */
  selecting: boolean;
  /** `range` is true with Shift held. */
  onCheck: (on: boolean, range: boolean) => void;
  className?: string;
}

/**
 * The select box on a library model, as the Board's cards have it: hidden
 * until a hover or a selection, and Shift picks the models between.
 */
export default function SelectBox({
  name,
  checked,
  selecting,
  onCheck,
  className,
}: SelectBoxProps) {
  return (
    <Tooltip label="Select for a bulk action" detail="Shift-click selects the models between">
      <button
        type="button"
        role="checkbox"
        aria-checked={checked}
        aria-label={`Select ${name}`}
        onClick={e => onCheck(!checked, e.shiftKey)}
        className={cn(
          'grid size-6 flex-none cursor-pointer place-items-center rounded-[6px] border-[1.5px] shadow-[0_1px_3px_rgba(0,0,0,0.35)] transition-opacity duration-120 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus',
          checked
            ? 'border-clay-text bg-clay text-clay-ink'
            : 'border-line-strong bg-panel/90 text-transparent hover:border-fg-mute',
          checked || selecting ? 'opacity-100' : 'opacity-0 group-hover/model:opacity-100',
          className
        )}
      >
        <Icon name="check" size={13} />
      </button>
    </Tooltip>
  );
}
