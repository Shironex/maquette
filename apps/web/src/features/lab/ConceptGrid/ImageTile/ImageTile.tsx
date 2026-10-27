import { Button, Icon, IconButton, Tooltip, cn } from '@maquette/ui';
import type { ConceptImage, Direction, LabView } from '@/ipc/client';
import { aspectCss, isObjects } from '@/state/lab';
import { useImageTile } from './ImageTile.hooks';

export interface ImageTileProps {
  view: LabView;
  direction: Direction;
  image: ConceptImage;
}

/** A finished concept: open it, pick it, star it, or make object refs from it. */
export default function ImageTile({ view, direction, image }: ImageTileProps) {
  const { mark, openLightbox, openObjects, path, url, label } = useImageTile(view, image);
  return (
    <figure className="group m-0 flex flex-col gap-[7px]">
      <div
        className={cn(
          'relative overflow-hidden rounded-[10px] bg-raised',
          image.picked
            ? 'shadow-[0_0_0_2px_var(--color-clay)]'
            : 'shadow-[0_0_0_1px_var(--color-line)]'
        )}
        style={{ aspectRatio: aspectCss(direction.aspect) }}
      >
        <Tooltip label="Opens it large" side="bottom">
          <button
            type="button"
            aria-label={`Open ${label}`}
            onClick={() => openLightbox(path)}
            className="block size-full cursor-zoom-in"
          >
            {url ? (
              <img
                src={url}
                alt={`${direction.name}, ${label}`}
                className="block size-full object-cover"
              />
            ) : null}
          </button>
        </Tooltip>
        {image.picked ? (
          <span className="pointer-events-none absolute top-2 left-2 inline-flex h-[22px] items-center gap-1 rounded-pill bg-clay px-2 text-[11.5px] font-semibold text-clay-ink">
            <Icon name="check" size={12} />
            Picked
          </span>
        ) : null}
        <Button
          size="sm"
          variant={image.picked ? 'secondary' : 'primary'}
          icon={image.picked ? 'x' : 'check'}
          tooltip={
            image.picked
              ? 'Takes this concept off the picked list'
              : 'Marks this concept to send to Refs'
          }
          onClick={() => void mark(image.file, { picked: !image.picked })}
          className="absolute top-2 right-2 opacity-0 transition-opacity duration-120 group-focus-within:opacity-100 group-hover:opacity-100"
        >
          {image.picked ? 'Unpick' : 'Pick'}
        </Button>
      </div>
      <figcaption className="flex items-center justify-between gap-2">
        <span className="min-w-0 truncate font-medium">{label}</span>
        <span className="flex flex-none items-center gap-1.5">
          {image.refFile ? (
            <span className="text-meta whitespace-nowrap text-fg-mute" title={image.refFile}>
              In Refs
            </span>
          ) : null}
          {isObjects(direction) ? null : (
            <IconButton
              icon="cube-focus"
              label={`Make object refs from ${label}`}
              hint="One image per object, each with its own notes"
              variant="ghost"
              size="sm"
              onClick={() =>
                openObjects({
                  project: view.project,
                  source: image.file,
                  sourcePath: path,
                  sourceName: direction.name,
                })
              }
              className="text-fg-mute"
            />
          )}
          <IconButton
            icon={image.starred ? 'star-fill' : 'star'}
            label={image.starred ? `Unstar ${label}` : `Star ${label}`}
            aria-pressed={image.starred}
            variant="ghost"
            size="sm"
            onClick={() => void mark(image.file, { starred: !image.starred })}
            className={image.starred ? 'text-clay-text' : 'text-fg-mute'}
          />
        </span>
      </figcaption>
    </figure>
  );
}
