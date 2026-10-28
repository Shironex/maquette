import type { ConceptImage, LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { inProject, tileLabel } from '@/state/lab';
import { MAX_STYLE_IMAGES } from '@/state/lab-style';
import { useLabStore } from '@/state/use-lab-store';

/** The image's path, URL and label, with the store actions its buttons call. */
export function useImageTile(view: LabView, image: ConceptImage) {
  const mark = useLabStore(s => s.mark);
  const openLightbox = useLabStore(s => s.openLightbox);
  const openObjects = useLabStore(s => s.openObjects);
  const addToStyle = useLabStore(s => s.addToStyle);
  const styleBusy = useLabStore(s => s.busy === 'style');
  const path = inProject(view.projectDir, image.file);
  const url = fileUrl(path);
  const label = tileLabel(image);
  const onBoard = view.style.images.some(i => i.source === image.file);
  const boardFull = view.style.images.length >= MAX_STYLE_IMAGES;
  return {
    mark,
    openLightbox,
    openObjects,
    addToStyle,
    styleBusy,
    path,
    url,
    label,
    onBoard,
    boardFull,
  };
}
