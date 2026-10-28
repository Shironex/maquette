import type { LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { inProject } from '@/state/lab';
import { isStyleEmpty, styleCounts } from '@/state/lab-style';
import { useLabStore } from '@/state/use-lab-store';

/** The board as the switch shows it: empty or not, its thumbnails, and the dialog's opener. */
export function useStyleSwitch(view: LabView) {
  const openStyle = useLabStore(s => s.openStyle);
  const board = view.style;
  const thumbs = board.images.map(i => ({
    file: i.file,
    url: fileUrl(inProject(view.projectDir, i.file)),
  }));
  return {
    board,
    empty: isStyleEmpty(board),
    counts: styleCounts(board),
    thumbs,
    openStyle,
  };
}
