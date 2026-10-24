import type { ProjectManifest, ProjectTemplate } from '@/ipc/client';
import diorama from '@/assets/moon-island.jpg';
import room from '@/assets/template-room.jpg';
import character from '@/assets/template-character.jpg';
import { TEMPLATE_LABEL } from '@/state/onboarding';

const THUMB: Partial<Record<ProjectTemplate, string>> = {
  'floating-diorama': diorama,
  room,
  'character-showcase': character,
};

/** Until a project has a poster, its template picture stands in. */
export function projectThumb(project: ProjectManifest): string | undefined {
  return THUMB[project.template];
}

/** The switcher subtitle: the site it ships to, else the template. */
export function projectMeta(project: ProjectManifest): string {
  if (project.siteRepo) {
    const parts = project.siteRepo.split(/[\\/]+/).filter(Boolean);
    return parts[parts.length - 1]?.replace(/[-_]+/g, ' ') ?? TEMPLATE_LABEL[project.template];
  }
  return TEMPLATE_LABEL[project.template];
}
