import type { ProjectManifest, ProjectSummary, ProjectTemplate } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import diorama from '@/assets/moon-island.jpg';
import room from '@/assets/template-room.jpg';
import character from '@/assets/template-character.jpg';
import { TEMPLATE_LABEL } from '@/state/onboarding';

const THUMB: Partial<Record<ProjectTemplate, string>> = {
  'floating-diorama': diorama,
  room,
  'character-showcase': character,
};

/**
 * The project's own picture (a poster or a card render) when the list has
 * one, else its template picture.
 */
export function projectThumb(
  project: Pick<ProjectManifest, 'template'>,
  summary?: Pick<ProjectSummary, 'dir' | 'thumb'>
): string | undefined {
  const own = summary?.thumb ? fileUrl(`${summary.dir}/${summary.thumb}`) : null;
  return own ?? THUMB[project.template];
}

/** The switcher subtitle: the site it ships to, else the template. */
export function projectMeta(project: ProjectManifest): string {
  if (project.siteRepo) {
    const parts = project.siteRepo.split(/[\\/]+/).filter(Boolean);
    return parts[parts.length - 1]?.replace(/[-_]+/g, ' ') ?? TEMPLATE_LABEL[project.template];
  }
  return TEMPLATE_LABEL[project.template];
}
