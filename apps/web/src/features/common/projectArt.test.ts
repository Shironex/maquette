import { afterEach, describe, expect, it } from 'vitest';
import { registerMockFile } from '@/ipc/files';
import { useAppStore } from '@/state/use-app-store';
import { TEMPLATE_LABEL } from '@/state/onboarding';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { projectMeta, projectThumb } from './projectArt';

afterEach(() => clearMocks());

async function project() {
  await openMockWorkspace();
  return useAppStore.getState().workspace!.project!;
}

describe('the project art', () => {
  it('names the site repo folder, else the template', async () => {
    const p = await project();
    expect(projectMeta({ ...p, siteRepo: '/Users/me/sites/moon_island-site/' })).toBe(
      'moon island site'
    );
    expect(projectMeta({ ...p, siteRepo: null })).toBe(TEMPLATE_LABEL[p.template]);
  });

  it('prefers the project picture from the list over the template picture', async () => {
    const p = await project();
    registerMockFile('/w/projects/moon/poster.jpg', 'poster-url');
    expect(projectThumb(p, { dir: '/w/projects/moon', thumb: 'poster.jpg' })).toBe('poster-url');
    expect(projectThumb(p, { dir: '/w/projects/moon', thumb: null })).not.toBe('poster-url');
  });
});
