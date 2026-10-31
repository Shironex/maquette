import { Icon, cn } from '@maquette/ui';
import type { PackItem } from '@/ipc/client';
import { KIND_ICON } from '@/state/thumbs';
import { usePackPicture } from './PackPicture.hooks';

export interface PackPictureProps {
  item: PackItem;
  className?: string;
}

/**
 * The render of what a pack wrote. A picture that will not load (a record
 * moved with its workspace, a file swept away) gives way to the library's
 * picture, then to the kind's icon, so a card never shows a broken image.
 */
export default function PackPicture({ item, className }: PackPictureProps) {
  const { url, onError } = usePackPicture(item);
  if (!url) return <Icon name={KIND_ICON[item.model.kind]} size={28} className="text-fg-mute" />;
  return (
    <img
      key={url}
      src={url}
      alt=""
      loading="lazy"
      decoding="async"
      className={cn('size-full object-contain', className)}
      onError={onError}
    />
  );
}
