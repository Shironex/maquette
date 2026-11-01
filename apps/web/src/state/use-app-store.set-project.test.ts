import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { ProjectManifest } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';

describe('a project an agent just made', () => {
  beforeEach(async () => {
    await openMockWorkspace();
  });
  afterEach(() => clearMocks());

  const made = (id: string) => ({ id, name: id, assets: [] }) as unknown as ProjectManifest;

  it('joins the switcher and leaves the open project alone', () => {
    const before = useAppStore.getState().workspace!;
    useAppStore.getState().setProject(made('agent-made'));
    const after = useAppStore.getState().workspace!;
    expect(after.project?.id).toBe(before.project?.id);
    expect(after.projects.map(p => p.id)).toContain('agent-made');
    expect(after.projects).toHaveLength(before.projects.length + 1);
  });

  it('opens when no project was open', () => {
    const workspace = useAppStore.getState().workspace!;
    useAppStore.setState({ workspace: { ...workspace, project: null, projects: [] } });
    useAppStore.getState().setProject(made('first'));
    expect(useAppStore.getState().workspace?.project?.id).toBe('first');
  });
});
