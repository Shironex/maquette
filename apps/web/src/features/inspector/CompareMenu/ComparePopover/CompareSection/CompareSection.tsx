import { Icon, cn } from '@maquette/ui';
import { optionDomId, type CompareGroup, type CompareOption } from '../../compare-options';

export interface CompareSectionProps {
  group: CompareGroup;
  listId: string;
  /** The option a key press would pick. */
  active: CompareOption | null;
  onPick: (option: CompareOption) => void;
}

/** One titled group of options in the compare list. */
export default function CompareSection({ group, listId, active, onPick }: CompareSectionProps) {
  const titleId = `${listId}-${group.id}`;
  const rows = group.options.map(option => (
    <div
      key={option.id}
      id={optionDomId(listId, option)}
      role="option"
      aria-selected={option === active}
      aria-current={option.current || undefined}
      aria-disabled={option.current || undefined}
      onPointerDown={e => e.preventDefault()}
      onClick={() => {
        if (!option.current) onPick(option);
      }}
      className={cn(
        'flex cursor-pointer items-center gap-2 rounded-control px-2 py-1.5 text-[13px] hover:bg-hover',
        option === active && 'bg-hover',
        option.current && 'cursor-default text-fg-mute hover:bg-transparent'
      )}
    >
      <Icon name={option.icon} size={14} className="flex-none text-fg-dim" />
      <span className="min-w-0 flex-1 truncate">{option.label}</span>
      {option.detail ? (
        <span className="flex-none text-[12px] text-fg-mute">{option.detail}</span>
      ) : null}
      {option.current ? <Icon name="check" size={12} className="flex-none text-fg-mute" /> : null}
    </div>
  ));
  return (
    <div role="group" aria-labelledby={titleId}>
      <div
        id={titleId}
        className="px-2 pt-2 pb-1 text-[11px] font-medium tracking-wide text-fg-mute uppercase"
      >
        {group.title}
      </div>
      {rows}
    </div>
  );
}
