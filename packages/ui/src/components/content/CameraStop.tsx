import { cn } from '../../lib/cn';

export interface CameraStopProps {
  src: string;
  label: string;
  index?: number;
  active?: boolean;
  onClick?: () => void;
  className?: string;
}

export function CameraStop({ src, label, index, active, onClick, className }: CameraStopProps) {
  return (
    <button
      type="button"
      aria-pressed={!!active}
      onClick={onClick}
      className={cn(
        'flex w-[132px] cursor-pointer flex-col gap-1.5 text-left text-[12px] text-fg-dim hover:text-fg',
        active && 'text-fg',
        className
      )}
    >
      <span
        className={cn(
          'relative aspect-video overflow-hidden rounded-control outline-1 -outline-offset-1 outline-line',
          active && 'outline-2 outline-offset-1 outline-clay'
        )}
      >
        <img src={src} alt="" className="block size-full object-cover" />
        {index !== undefined ? (
          <span className="absolute top-1.5 left-1.5 rounded-[4px] bg-scrim px-[5px] text-[11px] leading-4 text-fg tabular-nums">
            {index}
          </span>
        ) : null}
      </span>
      <span>{label}</span>
    </button>
  );
}
