import { boardPrimary } from '@/state/board';
import { useAppStore } from '@/state/use-app-store';
import { useFirstRunTips } from '@/state/use-first-run-tips';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** Which of the Board's header buttons is the bright one, from the project's state. */
export function useBoardPrimary() {
  const project = useAppStore(s => s.workspace?.project);
  const running = usePipelineStore(s => Boolean(s.view?.active));
  const tipsHidden = useFirstRunTips(s => s.hidden);
  return boardPrimary(project?.assets ?? [], {
    open: Boolean(project),
    running,
    tips: !tipsHidden,
  });
}
