import { useEffect, useMemo, useRef, type MouseEvent } from 'react';
import type { ConceptImage, LabView } from '@/ipc/client';
import { useModalFocus } from '@/lib/useModalFocus';
import { inProject } from '@/state/lab';
import { useLabStore } from '@/state/use-lab-store';

/** The image `by` steps from `at`, wrapping around the direction. */
const neighbour = (images: readonly ConceptImage[], at: number, by: number) =>
  images[(at + by + images.length) % images.length];

/** The open image within its direction, and the keys that step, star, pick and close. */
export function useLightbox(view: LabView) {
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

  useModalFocus(dialog, Boolean(path));
  useEffect(() => {
    if (!path) return;
    dialog.current?.querySelector<HTMLButtonElement>('[data-close]')?.focus();
    const step = (by: number) => {
      if (at < 0 || images.length < 2) return;
      const next = neighbour(images, at, by);
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

  const closeOnBackdrop = (e: MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) open(null);
  };
  /** The Previous and Next buttons. */
  const step = (by: number) => {
    const next = neighbour(images, at, by);
    if (next) open(inProject(view.projectDir, next.file));
  };

  return { path, open, mark, dialog, direction, images, at, image, closeOnBackdrop, step };
}
