import { clsx, type ClassValue } from 'clsx';
import { extendTailwindMerge } from 'tailwind-merge';
import { THEME_NAMES } from './theme-names';

// The kit's theme replaces Tailwind's palette and type scale with token names,
// so tailwind-merge has to learn them, or `text-body` and `text-fg` would be
// read as the same group and one would silently win.
const twMerge = extendTailwindMerge({
  extend: {
    theme: {
      color: [...THEME_NAMES.colors],
      text: [...THEME_NAMES.text],
      radius: [...THEME_NAMES.radius],
      shadow: [...THEME_NAMES.shadow],
    },
  },
});

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
