import type { IconName } from '@maquette/ui';

export type SectionId =
  'workspace' | 'toolchain' | 'providers' | 'images' | 'budgets' | 'agents' | 'about' | 'project';

export const SECTIONS: readonly { id: SectionId; label: string; icon: IconName }[] = [
  { id: 'workspace', label: 'Workspace', icon: 'folder' },
  { id: 'toolchain', label: 'Toolchain', icon: 'wrench' },
  { id: 'providers', label: '3D providers', icon: 'cube' },
  { id: 'images', label: 'Image generation', icon: 'image' },
  { id: 'budgets', label: 'Budgets and compression', icon: 'gauge' },
  { id: 'agents', label: 'Agents', icon: 'robot' },
  { id: 'about', label: 'About', icon: 'info' },
];

/** The ring on the section the nav last jumped to. */
export const ACTIVE = 'shadow-[0_0_0_1px_var(--clay-line)]';
