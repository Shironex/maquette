/**
 * A sheet's bottom button bar. It runs to the window's bottom edge, so the
 * floating approval cards and notices sit above it instead of over its
 * buttons. Sheet's `footer` renders one; a footer that lives deeper in a
 * sheet's body uses it directly.
 */
import type { ReactNode } from 'react';
import { cn } from '@maquette/ui';
import { useKeepClearOfFloating } from '@/lib/useFloatingClearance';

export default function SheetFooter({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  const bar = useKeepClearOfFloating<HTMLDivElement>();
  return (
    <div ref={bar} data-sheet-footer="" className={cn('flex-none', className)}>
      {children}
    </div>
  );
}
