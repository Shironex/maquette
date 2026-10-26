import { Icon, cn, type IconName } from '@maquette/ui';

export interface PressToggleProps {
  icon: IconName;
  label: string;
  pressed: boolean;
  onChange: (pressed: boolean) => void;
  disabled?: boolean;
  /** Why it is off limits, as its tooltip. */
  title?: string;
}

/** A toolbar button that stays pressed while its overlay shows. */
export default function PressToggle({
  icon,
  label,
  pressed,
  onChange,
  disabled,
  title,
}: PressToggleProps) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      disabled={disabled}
      title={title}
      onClick={() => onChange(!pressed)}
      className={cn(
        'inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-control border border-line px-2.5 text-[12px] leading-none font-medium whitespace-nowrap text-fg-dim hover:bg-hover disabled:cursor-default disabled:opacity-50',
        pressed && 'border-line-strong bg-hover text-fg'
      )}
    >
      <Icon name={icon} size={14} className={pressed ? 'text-clay-text' : undefined} />
      {label}
    </button>
  );
}
