import type { ButtonHTMLAttributes } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';
export type ButtonSize = 'sm' | 'md' | 'lg';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: IconName;
  iconRight?: IconName;
  loading?: boolean;
  block?: boolean;
}

const BASE =
  'inline-flex items-center justify-center whitespace-nowrap rounded-control border font-medium leading-none cursor-pointer select-none ' +
  'transition-[background-color,border-color,color,transform] duration-120 ease-out active:enabled:translate-y-px ' +
  'disabled:cursor-not-allowed disabled:opacity-45';

export const BUTTON_VARIANTS: Record<ButtonVariant, string> = {
  primary:
    'border-clay-text bg-clay font-semibold text-clay-ink hover:enabled:bg-clay-hover hover:enabled:border-clay-hover',
  secondary: 'border-line-strong bg-raised text-fg hover:enabled:bg-hover',
  ghost:
    'border-transparent bg-transparent text-fg-dim hover:enabled:bg-hover hover:enabled:text-fg',
  danger:
    'border-danger/45 bg-transparent text-danger hover:enabled:border-danger hover:enabled:bg-danger-soft',
};

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
  className,
  children,
  disabled,
  type = 'button',
  ...rest
}: ButtonProps) {
  const iconSize = size === 'md' ? 14 : size === 'sm' ? 14 : 15;
  return (
    <button
      type={type}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      className={cn(buttonClass(variant, size), block && 'w-full', className)}
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
}
