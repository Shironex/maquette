import { useEffect, useMemo, useRef } from 'react';
import { Button, IconButton } from '@maquette/ui';
import type { LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { inProject, tileLabel } from '@/state/lab';
import { useLabStore } from '@/state/use-lab-store';

/**
 * One image at full size over the Lab. Arrow keys step through the
 * direction, S stars, P picks, Escape closes. A contact sheet opens here
 * too, on its own.
 */
export function Lightbox({ view }: { view: LabView }) {
  const path = useLabStore(s => s.lightbox);
  const open = useLabStore(s => s.openLightbox);
  const mark = useLabStore(s => s.mark);
  const selected = useLabStore(s => s.selected);
  const dialog = useRef<HTMLDivElement>(null);
  const direction = view.directions.find(d => d.id === selected);
  const images = useMemo(
    () => (direction ? [...direction.images].sort((a, b) => a.index - b.index) : []),
    [direction]
  );
  const at = images.findIndex(i => inProject(view.projectDir, i.file) === path);
  const image = at >= 0 ? images[at] : undefined;

  useEffect(() => {
    if (!path) return;
    dialog.current?.querySelector<HTMLButtonElement>('[data-close]')?.focus();
    const step = (by: number) => {
      if (at < 0 || images.length < 2) return;
      const next = images[(at + by + images.length) % images.length];
      if (next) open(inProject(view.projectDir, next.file));
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') open(null);
      else if (e.key === 'ArrowRight') step(1);
      else if (e.key === 'ArrowLeft') step(-1);
      else if (image && (e.key === 's' || e.key === 'S'))
        void mark(image.file, { starred: !image.starred });
      else if (image && (e.key === 'p' || e.key === 'P'))
        void mark(image.file, { picked: !image.picked });
      else return;
      e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [path, at, images, image, open, mark, view.projectDir]);

  if (!path) return null;
  const url = fileUrl(path);
  const name = path.slice(path.lastIndexOf('/') + 1);
  const title = image ? `${direction?.name ?? ''} · ${tileLabel(image)}` : name;

  return (
    <div
      ref={dialog}
      role="dialog"
      aria-modal="true"
      aria-label={title}
      className="fixed inset-0 z-50 flex flex-col bg-[rgba(16,15,14,0.97)] backdrop-blur-sm"
      onClick={e => {
        if (e.target === e.currentTarget) open(null);
      }}
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
              {at + 1} of {images.length}
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
        onClick={e => {
          if (e.target === e.currentTarget) open(null);
        }}
      >
        {image && images.length > 1 ? (
          <IconButton
            icon="caret-left"
            label="Previous image"
            variant="secondary"
            className="absolute left-4"
            onClick={() => {
              const prev = images[(at - 1 + images.length) % images.length];
              if (prev) open(inProject(view.projectDir, prev.file));
            }}
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
            onClick={() => {
              const next = images[(at + 1) % images.length];
              if (next) open(inProject(view.projectDir, next.file));
            }}
          />
        ) : null}
      </div>
    </div>
  );
}
