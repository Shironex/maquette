import { Button, IconButton } from '@maquette/ui';
import type { LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { tileLabel } from '@/state/lab';
import { useLightbox } from './Lightbox.hooks';

export interface LightboxProps {
  view: LabView;
}

/**
 * One image at full size over the Lab. Arrow keys step through the
 * direction, S stars, P picks, Escape closes. A contact sheet opens here
 * too, on its own.
 */
export default function Lightbox({ view }: LightboxProps) {
  const { path, open, mark, dialog, direction, images, at, image, closeOnBackdrop, step } =
    useLightbox(view);
  if (!path) return null;
  const url = fileUrl(path);
  const name = path.slice(path.lastIndexOf('/') + 1);
  const title = image ? `${direction?.name ?? ''} · ${tileLabel(image)}` : name;
  const position = at + 1;

  return (
    <div
      ref={dialog}
      role="dialog"
      aria-modal="true"
      aria-label={title}
      className="fixed inset-0 z-50 flex flex-col bg-[rgba(16,15,14,0.97)] backdrop-blur-sm"
      onClick={closeOnBackdrop}
    >
      <header className="flex items-center gap-3 px-5 pt-4 pb-3">
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-panel-title">{title}</h2>
          <p className="truncate font-mono text-[12px] text-fg-mute">
            {image
              ? `${image.width && image.height ? `${image.width} × ${image.height} · ` : ''}${image.file}`
              : path}
          </p>
        </div>
        {image ? (
          <>
            <span className="text-meta text-fg-mute tabular-nums">
              {position} of {images.length}
            </span>
            <Button
              icon={image.starred ? 'star-fill' : 'star'}
              aria-pressed={image.starred}
              onClick={() => void mark(image.file, { starred: !image.starred })}
            >
              {image.starred ? 'Starred' : 'Star'}
            </Button>
            <Button
              variant={image.picked ? 'secondary' : 'primary'}
              icon="check"
              aria-pressed={image.picked}
              onClick={() => void mark(image.file, { picked: !image.picked })}
            >
              {image.picked ? 'Picked' : 'Pick'}
            </Button>
          </>
        ) : null}
        <IconButton data-close icon="x" label="Close" onClick={() => open(null)} />
      </header>
      <div
        className="relative flex min-h-0 flex-1 items-center justify-center px-16 pb-6"
        onClick={closeOnBackdrop}
      >
        {image && images.length > 1 ? (
          <IconButton
            icon="caret-left"
            label="Previous image"
            variant="secondary"
            className="absolute left-4"
            onClick={() => step(-1)}
          />
        ) : null}
        {url ? (
          <img
            src={url}
            alt={title}
            className="max-h-full max-w-full rounded-card object-contain shadow-window"
          />
        ) : (
          <p className="text-fg-mute">This image cannot be shown.</p>
        )}
        {image && images.length > 1 ? (
          <IconButton
            icon="caret-right"
            label="Next image"
            variant="secondary"
            className="absolute right-4"
            onClick={() => step(1)}
          />
        ) : null}
      </div>
    </div>
  );
}
