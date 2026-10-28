import { useRef, useState, type DragEvent } from 'react';
import { useWindowedList } from '@maquette/ui';
import type { Stage } from '@/ipc/client';
import { DRAG_TYPE } from '../constants';

/** Lights the column while a card is dragged over it and moves the card on drop. */
export function useColumnDrop(stage: Stage, dropOn: (stage: Stage, id: string) => void) {
  const [over, setOver] = useState(false);
  const onDragOver = (e: DragEvent) => {
    if (!e.dataTransfer.types.includes(DRAG_TYPE)) return;
    e.preventDefault();
    setOver(true);
  };
  const onDragLeave = () => setOver(false);
  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    setOver(false);
    dropOn(stage, e.dataTransfer.getData(DRAG_TYPE));
  };
  return { over, onDragOver, onDragLeave, onDrop };
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
