import { useRef, useState, type DragEvent, type RefObject } from 'react';
import { commands, type LabView } from '@/ipc/client';
import { IN_TAURI } from '@/ipc/runtime';
import { IMAGE_FILE, mockDroppedImages } from '@/lib/dropped-images';
import { useNativeDrop, type DropPoint } from '@/lib/useNativeDrop';
import { MAX_REFS } from '@/state/lab';
import { useLabStore } from '@/state/use-lab-store';

/** Native file drops over the composer column add refs. */
function useComposerDrop(
  zone: RefObject<HTMLDivElement | null>,
  setOver: (over: boolean) => void,
  onPaths: (paths: string[]) => void
) {
  const inside = ({ x, y }: DropPoint) => {
    const rect = zone.current?.getBoundingClientRect();
    if (!rect) return false;
    // The whole composer column accepts drops, not just the chips.
    return x >= rect.left - 18 && x <= rect.right + 18 && y >= 0;
  };
  useNativeDrop({
    onOver: at => setOver(inside(at)),
    onLeave: () => setOver(false),
    onDrop: (at, paths) => {
      setOver(false);
      if (inside(at)) onPaths(paths.filter(path => IMAGE_FILE.test(path)));
    },
  });
}

/** The draft's refs, the drop zone, and the Add menu with the unused refs/ images. */
export function useReferences(view: LabView) {
  const draft = useLabStore(s => s.draft);
  const addRefs = useLabStore(s => s.addRefs);
  const removeRef = useLabStore(s => s.removeRef);
  const [menu, setMenu] = useState(false);
  const [over, setOver] = useState(false);
  const zone = useRef<HTMLDivElement>(null);
  const full = draft.refs.length >= MAX_REFS;
  useComposerDrop(
    zone,
    setOver,
    paths => void addRefs(paths.slice(0, MAX_REFS - draft.refs.length))
  );

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    setOver(false);
    // A plain browser (mock mode) gets File objects without paths.
    if (IN_TAURI) return;
    void addRefs(mockDroppedImages(e.dataTransfer.files));
  };
  const onDragOver = (e: DragEvent) => {
    e.preventDefault();
    setOver(true);
  };
  const onDragLeave = () => setOver(false);

  const unused = view.refs.filter(r => !draft.refs.includes(r.file));
  const choose = async () => {
    setMenu(false);
    const path = await commands.dialogPickFile('Choose a reference image');
    if (path) await addRefs([path]);
  };
  const pick = (file: string) => {
    setMenu(false);
    void addRefs([file]);
  };

  return {
    refs: draft.refs,
    removeRef,
    menu,
    setMenu,
    over,
    zone,
    full,
    onDrop,
    onDragOver,
    onDragLeave,
    unused,
    choose,
    pick,
  };
}
