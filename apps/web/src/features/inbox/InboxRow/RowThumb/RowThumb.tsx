import { Icon, cn } from '@maquette/ui';
import type { InboxItem } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { KIND_ICON } from '@/state/thumbs';

export interface RowThumbProps {
  item: InboxItem;
  /** `sm` for a table row. */
  size?: 'md' | 'sm';
}

/** The studio render, or the kind icon while it renders (or for a file that cannot be read). */
export default function RowThumb({ item, size = 'md' }: RowThumbProps) {
  const src = fileUrl(item.thumbnail);
  const box = size === 'sm' ? 'size-8 rounded-[6px]' : 'size-[52px] rounded-[8px]';
  if (src) {
    return (
      <img
        src={src}
        alt=""
        draggable={false}
        className={cn(box, 'flex-none bg-studio object-cover')}
      />
    );
  }
  const rendering = item.state === 'ready' && item.thumbnailState === 'rendering';
  return (
    <span
      title={rendering ? 'Rendering a thumbnail' : undefined}
      className={cn(
        box,
        'grid flex-none place-items-center border border-line bg-raised text-fg-mute'
      )}
    >
      <Icon
        name={item.state === 'failed' ? 'warning-circle' : KIND_ICON[item.kind]}
        size={size === 'sm' ? 14 : 20}
        className={cn(rendering && 'animate-pulse motion-reduce:animate-none')}
      />
    </span>
  );
}
