import { useRef, useState, type DragEvent, type RefObject } from 'react';
import { useWindowedList } from '@maquette/ui';
import { commands, errorMessage, type ProjectManifest, type Stage } from '@/ipc/client';
import { mockDroppedFiles } from '@/lib/dropped-images';
import { useNativeDrop, type DropPoint } from '@/lib/useNativeDrop';
import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';
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
    if (takesFiles(e)) onImages?.(mockDroppedFiles(e.dataTransfer.files));
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

/** Imports still running, per project: the last to settle reads the project. */
const importsRunning = new Map<string, number>();

/** Answers can come in any order, so only a read after the last one is sure to hold them all. */
async function readAfterImports(
  projectId: string,
  stillOpen: () => boolean,
  setProject: (project: ProjectManifest) => void,
  notify: ReturnType<typeof useAppStore.getState>['notify']
) {
  const left = (importsRunning.get(projectId) ?? 1) - 1;
  if (left > 0) {
    importsRunning.set(projectId, left);
    return;
  }
  importsRunning.delete(projectId);
  if (!stillOpen()) return;
  try {
    const project = await commands.projectGet(projectId);
    if (stillOpen()) setProject(project);
  } catch (error) {
    notify({
      status: 'failed',
      title: 'Could not read the project again',
      detail: errorMessage(error),
    });
  }
}

/** Images someone already has become a Lab direction and a card at Concept. */
export function useImportConcepts() {
  const projectId = useAppStore(s => s.workspace?.project?.id);
  const setProject = useAppStore(s => s.setProject);
  const setView = useAppStore(s => s.setView);
  const notify = useAppStore(s => s.notify);
  const importImages = async (paths: readonly string[]) => {
    if (!projectId || paths.length === 0) return;
    const stillOpen = () => useAppStore.getState().workspace?.project?.id === projectId;
    importsRunning.set(projectId, (importsRunning.get(projectId) ?? 0) + 1);
    try {
      // The core decides what is an image, by its header, and stops at the first that is not.
      const done = await commands.labImportConcepts(projectId, [...paths]);
      if (stillOpen()) void useLabStore.getState().load(projectId);
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
    } finally {
      await readAfterImports(projectId, stillOpen, setProject, notify);
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
