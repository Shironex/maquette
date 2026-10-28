import { Icon, cn } from '@maquette/ui';
import type { AssetChoice } from '@/state/downloads';
import { useChoicePicture } from './ChoiceRow.hooks';

export interface ChoiceRowProps {
  id: string;
  choice: AssetChoice;
  active: boolean;
  /** The asset the watcher thought it was. */
  suggested: boolean;
  onPick: () => void;
}

/** One asset in the chooser: its ref picture, name and where it stands. */
export default function ChoiceRow({ id, choice, active, suggested, onPick }: ChoiceRowProps) {
  const picture = useChoicePicture(choice.picture);
  return (
    <div
      id={id}
      role="option"
      aria-selected={active}
      className={cn(
        'flex cursor-pointer items-center gap-3 border-t border-line px-3 py-2 first:border-t-0',
        active ? 'bg-clay-soft' : 'hover:bg-studio'
      )}
      onPointerDown={e => e.preventDefault()}
      onClick={onPick}
    >
      <span className="grid size-9 flex-none place-items-center overflow-hidden rounded-control bg-studio text-fg-mute">
        {picture ? (
          <img src={picture} alt="" className="size-full object-cover" />
        ) : (
          <Icon name="image" size={15} />
        )}
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate font-medium">{choice.name}</span>
        <span
          className={cn(
            'block truncate text-meta',
            choice.waiting ? 'text-clay-text' : 'text-fg-mute'
          )}
        >
          {choice.state}
          {suggested ? ' · the name looked like it' : ''}
        </span>
      </span>
      {choice.waiting ? <Icon name="clock" size={14} className="text-clay-text" /> : null}
    </div>
  );
}
