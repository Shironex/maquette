import { Icon, cn } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { pictureOf } from '@/state/library';
import { KIND_ICON } from '@/state/thumbs';

export interface LibraryPictureProps {
  item: LibraryItem;
  className?: string;
}

/** An entry's newest render or ref, or its kind icon while it has none. */
export default function LibraryPicture({ item, className }: LibraryPictureProps) {
  const url = fileUrl(pictureOf(item));
  return (
    <div className={cn('grid place-items-center overflow-hidden bg-studio', className)}>
      {url ? (
        <img src={url} alt="" className="size-full object-contain" />
      ) : (
        <Icon name={KIND_ICON[item.entry.kind]} size={28} className="text-fg-mute" />
      )}
    </div>
  );
}
