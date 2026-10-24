import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { Monogram } from '../brand/Monogram';

export interface ProjectSwitcherProps {
  name: string;
  meta?: string;
  thumb?: string;
  compact?: boolean;
  onClick?: () => void;
  className?: string;
}

export function ProjectSwitcher({
  name,
  meta,
  thumb,
  compact,
  onClick,
  className,
}: ProjectSwitcherProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={compact ? `Project: ${name}` : undefined}
      title={compact ? name : undefined}
      className={cn(
        'flex w-full cursor-pointer items-center gap-2.5 rounded-card border border-line bg-raised p-2 text-left transition-colors duration-120 hover:border-line-strong',
        compact && 'w-11 p-[5px]',
        className
      )}
    >
      {thumb ? (
        <img src={thumb} alt="" className="size-[34px] flex-none rounded-[7px] object-cover" />
      ) : (
        <span className="grid size-[34px] flex-none place-items-center rounded-[7px] bg-hover">
          <Monogram size={22} label="" />
        </span>
      )}
      {compact ? null : (
        <>
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-[13px] leading-4 font-semibold">{name}</span>
            {meta ? (
              <span className="truncate text-[11.5px] leading-[15px] text-fg-mute">{meta}</span>
            ) : null}
          </span>
          <Icon name="caret-down" size={14} className="text-fg-mute" />
        </>
      )}
    </button>
  );
}
