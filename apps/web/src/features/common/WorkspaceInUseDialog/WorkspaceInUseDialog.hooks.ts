import { useWorkspaceLock } from '@/state/use-workspace-lock';

/** The open question, if any, and its two answers. */
export function useWorkspaceInUseDialog() {
  const question = useWorkspaceLock(s => s.question);
  const answer = useWorkspaceLock(s => s.answer);
  return {
    question,
    openAnyway: () => answer(true),
    cancel: () => answer(false),
  };
}
