import { Icon, cn } from '@maquette/ui';
import type { Arrival } from '@/ipc/client';
import { useDownloadThumb } from './DownloadThumb.hooks';

export interface DownloadThumbProps {
  arrival: Arrival;
  /** The square's size class. */
  className?: string;
}

/** A download's model, rendered headless; its file icon until the render is there. */
export default function DownloadThumb({ arrival, className = 'size-9' }: DownloadThumbProps) {
  const { url, rendering } = useDownloadThumb(arrival);
  const zip = arrival.name.toLowerCase().endsWith('.zip');
  return (
    <span
      className={cn(
        'grid flex-none place-items-center overflow-hidden rounded-control bg-studio text-fg-mute',
        className
      )}
      title={rendering ? 'Rendering a thumbnail' : undefined}
    >
      {url ? (
        <img src={url} alt={`The model in ${arrival.name}`} className="size-full object-cover" />
      ) : (
        <Icon name={zip ? 'package' : 'cube'} size={17} />
      )}
    </span>
  );
}
