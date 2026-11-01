import { useRef, useState, type DragEvent, type RefObject } from 'react';
import { useWindowedList } from '@maquette/ui';
import { commands, errorMessage, type Stage } from '@/ipc/client';
import { IMAGE_FILE, mockDroppedImages } from '@/lib/dropped-images';
import { useNativeDrop, type DropPoint } from '@/lib/useNativeDrop';
import { useAppStore } from '@/state/use-app-store';
import { DRAG_TYPE } from '../constants';

/** Lights the column under a dragged card, or files when it takes them, and takes the drop. */
export function useColumnDrop(
  stage: Stage,
  dropOn: (stage: Stage, id: string) => void,
  onImages?: (paths: string[]) => void
) {
  const [over, setOver] = useState(false);
  const takesFiles = (e: DragEvent) =>
    onImages && Array.from(e.dataTransfer.types).includes('Files');
  const onDragOver = (e: DragEvent) => {
    if (!e.dataTransfer.types.includes(DRAG_TYPE) && !takesFiles(e)) return;
    e.preventDefault();
    setOver(true);
  };
  const onDragLeave = () => setOver(false);
  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    setOver(false);
    if (takesFiles(e)) onImages?.(mockDroppedImages(e.dataTransfer.files));
    else dropOn(stage, e.dataTransfer.getData(DRAG_TYPE));
  };
  return { over, setOver, onDragOver, onDragLeave, onDrop };
}

/** A card's height before it is measured; its status and buttons make it grow. */
const CARD = 212;

/** The column's scroller and its cards, windowed once the column is long. */
export function useColumnCards(count: number) {
  const scrollRef = useRef<HTMLElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const cards = useWindowedList({ count, scrollRef, listRef, estimate: CARD, gap: 8 });
  return { scrollRef, listRef, cards };
}

/** Images someone already has become a Lab direction and a card at Concept. */
export function useImportConcepts() {
  const projectId = useAppStore(s => s.workspace?.project?.id);
  const setProject = useAppStore(s => s.setProject);
  const setView = useAppStore(s => s.setView);
  const notify = useAppStore(s => s.notify);
  const importImages = async (paths: readonly string[]) => {
    const images = paths.filter(p => IMAGE_FILE.test(p));
    if (!projectId) return;
    if (images.length === 0) {
      if (paths.length > 0) {
        notify({
          status: 'info',
          title: 'Drop a PNG, JPEG or WebP image',
          detail: 'The Concept column takes images. Models go in through the Inbox.',
        });
      }
      return;
    }
    try {
      const done = await commands.labImportConcepts(projectId, images);
      setProject(done.project);
      const n = done.added.length;
      notify({
        status: 'done',
        title: `${n} ${n === 1 ? 'image' : 'images'} added to ${done.direction.name}`,
        detail: 'Pick the ones you want in the Lab and send them to Refs.',
        action: { label: 'Open Lab', run: () => setView('lab') },
      });
    } catch (error) {
      notify({
        status: 'failed',
        title: 'Could not import those images',
        detail: errorMessage(error),
      });
    }
  };
  const chooseImages = async () => {
    const picked = await commands.dialogPickFiles('Choose concept art');
    await importImages(picked);
  };
  return { importImages, chooseImages };
}

/** Native file drops anywhere on the column, cards or not. A By hand card keeps its own. */
export function useNativeImageDrop(
  column: RefObject<HTMLElement | null>,
  enabled: boolean,
  setOver: (over: boolean) => void,
  onImages: (paths: string[]) => void
) {
  const inside = ({ x, y }: DropPoint) => {
    const under = document.elementFromPoint(x, y);
    return Boolean(
      enabled &&
      column.current &&
      under &&
      column.current.contains(under) &&
      !under.closest('[data-drop-job]')
    );
  };
  useNativeDrop({
    onOver: at => {
      if (enabled) setOver(inside(at));
    },
    onLeave: () => {
      if (enabled) setOver(false);
    },
    onDrop: (at, paths) => {
      if (!enabled) return;
      setOver(false);
      if (inside(at)) onImages(paths);
    },
  });
}
