import { useRetargetStore } from '@/state/use-retarget-store';

/** The open retarget and the choice of what a bone only on the model follows. */
export function useOnlyBones() {
  const sheet = useRetargetStore(s => s.sheet);
  const setFollow = useRetargetStore(s => s.setFollow);
  return { sheet, setFollow };
}
