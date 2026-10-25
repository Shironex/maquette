import type { ButtonHTMLAttributes } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { Tooltip } from '../feedback/Tooltip';
import type { TooltipSide } from '../feedback/tooltipPlacement';
import { blockClick, buttonClass, type ButtonVariant } from './Button';

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: IconName;
  /** The accessible name, also shown as the tooltip. */
  label: string;
  variant?: ButtonVariant;
  size?: 'sm' | 'md';
  /** The keys that do the same, shown in the tooltip. */
  shortcut?: string | readonly string[];
  /** Why it is disabled: it stays focusable and the tooltip says why. */
  disabledReason?: string;
  tooltipSide?: TooltipSide;
}

export function IconButton({
  icon,
  label,
  variant = 'ghost',
  size = 'md',
  shortcut,
  disabledReason,
  tooltipSide,
  className,
  disabled,
  onClick,
  type = 'button',
  ...rest
}: IconButtonProps) {
  const explained = Boolean(disabled && disabledReason);
  return (
    <Tooltip
      label={label}
      detail={explained ? disabledReason : undefined}
      shortcut={shortcut}
      side={tooltipSide}
    >
      <button
        type={type}
        aria-label={label}
        disabled={disabled && !explained}
        aria-disabled={explained || undefined}
        className={cn(
          buttonClass(variant, size),
          size === 'sm' ? 'w-[26px] px-0' : 'w-8 px-0',
          className
        )}
        onClick={explained ? blockClick : onClick}
        {...rest}
      >
        <Icon name={icon} size={size === 'sm' ? 14 : 16} />
      </button>
    </Tooltip>
  );
}
