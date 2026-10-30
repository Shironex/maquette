import { Icon, cn } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { BADGE } from '@/features/viewer/previewModes';
import { pictureOf } from '@/state/library';
import { KIND_ICON } from '@/state/thumbs';

export interface LibraryPictureProps {
  item: LibraryItem;
  className?: string;
  /** Opens the model in the 3D quick view; the picture is then a button. */
  onInspect?: () => void;
}

/**
 * An entry's newest render or ref, or its kind icon while it has none. With
 * `onInspect` a click on it opens the 3D quick view, and a hover says so.
 */
export default function LibraryPicture({ item, className, onInspect }: LibraryPictureProps) {
  const url = fileUrl(pictureOf(item));
  const picture = url ? (
    <img src={url} alt="" className="size-full object-contain" />
  ) : (
    <Icon name={KIND_ICON[item.entry.kind]} size={28} className="text-fg-mute" />
  );
  const frame = 'grid place-items-center overflow-hidden bg-studio';
  if (!onInspect) return <div className={cn(frame, className)}>{picture}</div>;
  return (
    <button
      type="button"
      aria-label={`Inspect 3D model of ${item.entry.name}`}
      onClick={onInspect}
      className={cn(
        frame,
        'group/picture relative cursor-zoom-in focus-visible:outline-offset-[-2px]',
        className
      )}
    >
      {picture}
      <span
        className={cn(
          BADGE,
          'right-2 bottom-2 opacity-0 transition-opacity duration-120 group-hover/picture:opacity-100 group-focus-visible/picture:opacity-100'
        )}
      >
        <Icon name="cube-transparent" size={12} />
        Inspect 3D
      </span>
    </button>
  );
}
