import type { ButtonHTMLAttributes, MouseEvent } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { Tooltip } from '../feedback/Tooltip';
import type { TooltipSide } from '../feedback/tooltipPlacement';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';
export type ButtonSize = 'sm' | 'md' | 'lg';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: IconName;
  iconRight?: IconName;
  loading?: boolean;
  block?: boolean;
  /**
   * Why it is disabled. The button stays focusable (aria-disabled) and its
   * tooltip says why, so the reason is there for the pointer and the keyboard.
   */
  disabledReason?: string;
  /** A tooltip, only when it adds something the label does not say. */
  tooltip?: string;
  /** The keys that do the same, shown in the tooltip. */
  shortcut?: string | readonly string[];
  tooltipSide?: TooltipSide;
}

const BASE =
  'inline-flex items-center justify-center whitespace-nowrap rounded-control border font-medium leading-none cursor-pointer select-none ' +
  'transition-[background-color,border-color,color,transform] duration-120 ease-out active:usable:translate-y-px ' +
  'disabled:cursor-not-allowed disabled:opacity-45 aria-disabled:cursor-not-allowed aria-disabled:opacity-45 aria-busy:cursor-progress';

export const BUTTON_VARIANTS: Record<ButtonVariant, string> = {
  primary:
    'border-clay-text bg-clay font-semibold text-clay-ink hover:usable:bg-clay-hover hover:usable:border-clay-hover active:usable:bg-clay',
  secondary:
    'border-line-strong bg-raised text-fg hover:usable:bg-hover active:usable:border-fg-mute',
  ghost:
    'border-transparent bg-transparent text-fg-dim hover:usable:bg-hover hover:usable:text-fg active:usable:bg-line',
  danger:
    'border-danger/45 bg-transparent text-danger hover:usable:border-danger hover:usable:bg-danger-soft',
};

/** A disabled-with-reason control takes focus and hover but not clicks. */
export function blockClick(e: MouseEvent) {
  e.preventDefault();
  e.stopPropagation();
}

export const BUTTON_SIZES: Record<ButtonSize, string> = {
  sm: 'h-[26px] gap-1.5 px-2.5 text-[12px]',
  md: 'h-8 gap-[7px] px-[13px] text-[13px]',
  lg: 'h-10 gap-2 rounded-[8px] px-[18px] text-[14px]',
};

export function buttonClass(variant: ButtonVariant = 'secondary', size: ButtonSize = 'md') {
  return cn(BASE, BUTTON_VARIANTS[variant], BUTTON_SIZES[size]);
}

export function Button({
  variant = 'secondary',
  size = 'md',
  icon,
  iconRight,
  loading,
  block,
  disabledReason,
  tooltip,
  shortcut,
  tooltipSide,
  className,
  children,
  disabled,
  onClick,
  type = 'button',
  ...rest
}: ButtonProps) {
  const iconSize = size === 'md' ? 14 : size === 'sm' ? 14 : 15;
  // Loading blocks for real, so a second press cannot submit twice.
  const explained = Boolean(disabled && disabledReason && !loading);
  const tip = explained ? disabledReason : tooltip;
  const button = (
    <button
      type={type}
      disabled={(disabled || loading) && !explained}
      aria-disabled={explained || undefined}
      aria-busy={loading || undefined}
      className={cn(buttonClass(variant, size), block && 'w-full', className)}
      onClick={explained ? blockClick : onClick}
      {...rest}
    >
      {loading ? (
        <Icon name="circle-notch" size={iconSize} spin />
      ) : icon ? (
        <Icon name={icon} size={iconSize} />
      ) : null}
      {children}
      {iconRight ? <Icon name={iconRight} size={iconSize} /> : null}
    </button>
  );
  return (
    <Tooltip
      label={tip ?? children}
      shortcut={explained ? undefined : shortcut}
      side={tooltipSide}
      wrap
      disabled={!tip && !shortcut}
    >
      {button}
    </Tooltip>
  );
}
