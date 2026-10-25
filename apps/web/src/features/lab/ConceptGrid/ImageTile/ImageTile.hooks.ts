import type { ConceptImage, LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { inProject, tileLabel } from '@/state/lab';
import { useLabStore } from '@/state/use-lab-store';

/** The image's path, URL and label, with the store actions its buttons call. */
export function useImageTile(view: LabView, image: ConceptImage) {
  const mark = useLabStore(s => s.mark);
  const openLightbox = useLabStore(s => s.openLightbox);
  const openObjects = useLabStore(s => s.openObjects);
  const path = inProject(view.projectDir, image.file);
  const url = fileUrl(path);
  const label = tileLabel(image);
  return { mark, openLightbox, openObjects, path, url, label };
}
