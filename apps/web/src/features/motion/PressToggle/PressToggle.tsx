import { Icon, Tooltip, cn, type IconName } from '@maquette/ui';

export interface PressToggleProps {
  icon: IconName;
  label: string;
  pressed: boolean;
  onChange: (pressed: boolean) => void;
  disabled?: boolean;
  /** What it shows or hides, as its tooltip. */
  tooltip?: string;
  /** Why it is off limits, as its tooltip while disabled. */
  title?: string;
}

/** A toolbar button that stays pressed while its overlay shows. */
export default function PressToggle({
  icon,
  label,
  pressed,
  onChange,
  disabled,
  tooltip,
  title,
}: PressToggleProps) {
  const explained = Boolean(disabled && title);
  const tip = explained ? title : tooltip;
  return (
    <Tooltip label={tip} disabled={!tip}>
      <button
        type="button"
        aria-pressed={pressed}
        disabled={disabled && !explained}
        aria-disabled={explained || undefined}
        onClick={() => {
          if (!explained) onChange(!pressed);
        }}
        className={cn(
          'inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-control border border-line px-2.5 text-[12px] leading-none font-medium whitespace-nowrap text-fg-dim hover:bg-hover disabled:cursor-default disabled:opacity-50 aria-disabled:cursor-default aria-disabled:opacity-50',
          pressed && 'border-line-strong bg-hover text-fg'
        )}
      >
        <Icon name={icon} size={14} className={pressed ? 'text-clay-text' : undefined} />
        {label}
      </button>
    </Tooltip>
  );
}
