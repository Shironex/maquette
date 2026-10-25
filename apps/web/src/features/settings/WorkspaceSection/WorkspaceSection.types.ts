import type { WorkspaceConfig } from '@/ipc/client';

export interface WorkspaceSectionProps {
  active: boolean;
  config: WorkspaceConfig;
}

export interface RootNote {
  tone: 'found' | 'missing';
  text: string;
}
