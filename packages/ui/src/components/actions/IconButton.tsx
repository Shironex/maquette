import type { ButtonHTMLAttributes } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { buttonClass, type ButtonVariant } from './Button';

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: IconName;
  /** The accessible name, also shown as the native tooltip. */
  label: string;
  variant?: ButtonVariant;
  size?: 'sm' | 'md';
}

export function IconButton({
  icon,
  label,
  variant = 'ghost',
  size = 'md',
  className,
  type = 'button',
  ...rest
}: IconButtonProps) {
  return (
    <button
      type={type}
      aria-label={label}
      title={label}
      className={cn(
        buttonClass(variant, size),
        size === 'sm' ? 'w-[26px] px-0' : 'w-8 px-0',
        className
      )}
      {...rest}
    >
      <Icon name={icon} size={size === 'sm' ? 14 : 16} />
    </button>
  );
}
