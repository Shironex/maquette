import { useState } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { useRigStore } from '@/state/use-rig-store';

/** The rig store's answers, and the project an Inbox model would be accepted into. */
export function useRigActions() {
  const rig = useRigStore();
  const project = useAppStore(s => s.workspace?.project?.id);
  return { ...rig, project };
}

/** A choice a rig doctor view keeps for itself: the sheet view, the compare mode. */
export function useChoice<T>(first: T): [T, (next: T) => void] {
  return useState<T>(first);
}
