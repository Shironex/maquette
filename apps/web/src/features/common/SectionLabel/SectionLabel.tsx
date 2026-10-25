import { cn } from '@maquette/ui';
import type { SectionLabelProps } from './SectionLabel.types';

export default function SectionLabel({ children, className }: SectionLabelProps) {
  return (
    <p className={cn('text-caption tracking-[0.04em] text-fg-mute uppercase', className)}>
      {children}
    </p>
  );
}
