import { Icon, cn } from '@maquette/ui';
import type { InboxItem } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { KIND_ICON } from '@/state/thumbs';

export interface RowThumbProps {
  item: InboxItem;
}

/** The studio render, or the kind icon while it renders (or for a file that cannot be read). */
export default function RowThumb({ item }: RowThumbProps) {
  const src = fileUrl(item.thumbnail);
  if (src) {
    return (
      <img
        src={src}
        alt=""
        draggable={false}
        className="size-[52px] flex-none rounded-[8px] bg-studio object-cover"
      />
    );
  }
  const rendering = item.state === 'ready' && item.thumbnailState === 'rendering';
  return (
    <span
      title={rendering ? 'Rendering a thumbnail' : undefined}
      className="grid size-[52px] flex-none place-items-center rounded-[8px] border border-line bg-raised text-fg-mute"
    >
      <Icon
        name={item.state === 'failed' ? 'warning-circle' : KIND_ICON[item.kind]}
        size={20}
        className={cn(rendering && 'animate-pulse motion-reduce:animate-none')}
      />
    </span>
  );
}
