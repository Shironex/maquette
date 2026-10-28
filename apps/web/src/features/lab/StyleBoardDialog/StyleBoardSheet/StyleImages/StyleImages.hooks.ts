import { useState } from 'react';
import { commands, type LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { inProject, tileLabel } from '@/state/lab';
import { MAX_STYLE_IMAGES } from '@/state/lab-style';

/** The most project images the Add menu offers. */
const MAX_OFFERED = 16;

const baseName = (path: string) => path.slice(path.replace(/\\/g, '/').lastIndexOf('/') + 1);

/** One image of the draft: its picture and where it came from. */
export interface DraftImage {
  file: string;
  url: string | null;
  name: string;
}

/**
 * The draft's images with their pictures, the project images the Add menu
 * offers (refs, then starred and picked concepts) and Choose a file.
 */
export function useStyleImages(
  view: LabView,
  images: readonly string[],
  onAdd: (file: string) => void
) {
  const [menu, setMenu] = useState(false);
  const sourceOf = (file: string) => view.style.images.find(i => i.file === file)?.source ?? null;
  const shown: DraftImage[] = images.map(file => ({
    file,
    url: fileUrl(inProject(view.projectDir, file)),
    name: baseName(sourceOf(file) ?? file),
  }));
  const taken = new Set([...images, ...images.map(sourceOf).filter(Boolean)]);
  const concepts = view.directions.flatMap(d =>
    d.images
      .filter(i => i.starred || i.picked)
      .map(i => ({ file: i.file, name: `${d.name}, ${tileLabel(i)}` }))
  );
  const offered = [...view.refs.map(r => ({ file: r.file, name: r.name })), ...concepts]
    .filter(o => !taken.has(o.file))
    .slice(0, MAX_OFFERED)
    .map(o => ({ ...o, url: fileUrl(inProject(view.projectDir, o.file)) }));
  const full = images.length >= MAX_STYLE_IMAGES;

  const pick = (file: string) => {
    setMenu(false);
    onAdd(file);
  };
  const choose = async () => {
    setMenu(false);
    const path = await commands.dialogPickFile('Choose a style image');
    if (path) onAdd(path);
  };
  return { shown, offered, full, menu, setMenu, pick, choose };
}
