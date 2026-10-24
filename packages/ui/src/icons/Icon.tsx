import type { SVGProps } from 'react';
import { cn } from '../lib/cn';
import { ICON_PATHS, type IconName } from './paths';

export type { IconName };

export interface IconProps extends Omit<SVGProps<SVGSVGElement>, 'name'> {
  name: IconName;
  size?: number;
  spin?: boolean;
  /** Gives the icon an accessible name; without it the icon is decorative. */
  title?: string;
}

export function Icon({ name, size = 16, spin, title, className, ...rest }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 256 256"
      fill="currentColor"
      aria-hidden={title ? undefined : true}
      role={title ? 'img' : undefined}
      className={cn('inline-block flex-none align-middle', spin && 'animate-spin', className)}
      {...rest}
    >
      {title ? <title>{title}</title> : null}
      <path d={ICON_PATHS[name]} />
    </svg>
  );
}

export const ICON_NAMES = Object.keys(ICON_PATHS) as IconName[];
