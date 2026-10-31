import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export interface LoaderProps {
  /** What is on its way; a screen reader hears it, and it stays when motion is reduced. */
  label?: string;
  className?: string;
}

/** The one spinner for anything still on its way: a view, its data or a 3D stage. */
export function Loader({ label = 'Loading...', className }: LoaderProps) {
  return (
    <div
      role="status"
      data-view-loader
      className={cn('flex flex-col items-center gap-2 text-meta text-fg-mute', className)}
    >
      <Icon name="circle-notch" size={20} spin className="text-clay-text" />
      {label}
    </div>
  );
}
