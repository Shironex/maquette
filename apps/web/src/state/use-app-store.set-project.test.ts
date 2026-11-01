import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { ProjectManifest } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';

describe('projects an event names', () => {
  beforeEach(async () => {
    await openMockWorkspace();
  });
  afterEach(() => clearMocks());

  const made = (id: string) => ({ id, name: id, assets: [] }) as unknown as ProjectManifest;

  it('a change to an unknown project, like a late thumbnail, adds nothing', () => {
    const before = useAppStore.getState().workspace!;
    useAppStore.getState().setProject(made('from-another-workspace'));
    const after = useAppStore.getState().workspace!;
    expect(after.projects).toEqual(before.projects);
    expect(after.project?.id).toBe(before.project?.id);
  });

  it('a project an agent made joins the switcher and leaves the open project alone', () => {
    const before = useAppStore.getState().workspace!;
    useAppStore.getState().addProject(made('agent-made'));
    useAppStore.getState().addProject(made('agent-made'));
    const after = useAppStore.getState().workspace!;
    expect(after.project?.id).toBe(before.project?.id);
    expect(after.projects).toHaveLength(before.projects.length + 1);
  });

  it('a project an agent made opens when none was open', () => {
    const workspace = useAppStore.getState().workspace!;
    useAppStore.setState({ workspace: { ...workspace, project: null, projects: [] } });
    useAppStore.getState().addProject(made('first'));
    expect(useAppStore.getState().workspace?.project?.id).toBe('first');
  });
});
