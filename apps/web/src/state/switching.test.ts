import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { inboxItems } from '@/ipc/mock/inbox-fixtures';
import { useAppStore } from './use-app-store';
import { acceptTarget, sentFor } from './inbox';
import { useLabStore } from './use-lab-store';
import { usePipelineStore } from './use-pipeline-store';
import { useProjectsStore } from './use-projects-store';
import { useProvidersStore } from './use-providers-store';

/** What useLiveData does when the open project changes. */
async function follow(project: string) {
  await Promise.all([
    usePipelineStore.getState().load(project),
    useLabStore.getState().load(project),
    useProvidersStore.getState().load(project),
  ]);
}

afterEach(() => {
  clearMocks();
  useLabStore.setState({ view: null, selected: null, lightbox: null, busy: null });
  usePipelineStore.setState({ view: null });
  useProvidersStore.setState({
    project: null,
    jobs: [],
    panel: null,
    batch: null,
    downloads: null,
  });
});

describe('switching projects', () => {
  it('starts every view of the new project clean', async () => {
    await openMockWorkspace('running');
    await follow('frostveil');
    expect(usePipelineStore.getState().view?.runs.length).toBeGreaterThan(0);
    const lab = useLabStore.getState();
    expect(lab.selected).not.toBeNull();
    expect(lab.draft.name).not.toBe('');
    useLabStore.setState({ lightbox: '/somewhere/concept.png' });
    await useProvidersStore.getState().openPanel('frostveil', 'academy-landing');
    expect(useProvidersStore.getState().panel?.project).toBe('frostveil');

    expect(await useProjectsStore.getState().switchTo('frost-cathedral')).toBe(true);
    await follow('frost-cathedral');

    expect(useAppStore.getState().workspace?.project?.id).toBe('frost-cathedral');
    const pipeline = usePipelineStore.getState().view;
    expect(pipeline?.project).toBe('frost-cathedral');
    expect(pipeline?.runs).toEqual([]);
    expect(pipeline?.active).toBeNull();
    const after = useLabStore.getState();
    expect(after.view?.project).toBe('frost-cathedral');
    expect(after.lightbox).toBeNull();
    expect(after.view?.directions.some(d => d.id === lab.selected)).toBe(false);
    expect(useProvidersStore.getState().project).toBe('frost-cathedral');
    expect(useProvidersStore.getState().panel).toBeNull();
  });

  it('closes the batch sheet and the downloads of the project left', async () => {
    await openMockWorkspace('configured', undefined, 'batch');
    await follow('frostveil');
    await useProvidersStore.getState().openBatch('frostveil', ['lantern-post', 'sigil-door']);
    await useProvidersStore.getState().checkDownloads();
    expect(useProvidersStore.getState().batch?.project).toBe('frostveil');
    expect(useProvidersStore.getState().downloads?.project).toBe('frostveil');

    await useProjectsStore.getState().switchTo('frost-cathedral');
    await follow('frost-cathedral');
    expect(useProvidersStore.getState().batch).toBeNull();
    expect(useProvidersStore.getState().downloads).toBeNull();
  });

  it('drops an answer for the project it just left', async () => {
    await openMockWorkspace('running');
    const slow = usePipelineStore.getState().load('frostveil');
    const lab = useLabStore.getState().load('frostveil');
    await useProjectsStore.getState().switchTo('frost-cathedral');
    await follow('frost-cathedral');
    await Promise.all([slow, lab]);
    expect(usePipelineStore.getState().view?.project).toBe('frost-cathedral');
    expect(useLabStore.getState().view?.project).toBe('frost-cathedral');
  });

  it('keeps a presets save for the project left out of the open one', async () => {
    const handle = await openMockWorkspace();
    // The save answers after the switch landed.
    mockIPC(async (cmd, args) => {
      if (cmd === 'pipeline_save_presets') await new Promise(r => setTimeout(r, 20));
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    await usePipelineStore.getState().load('frostveil');
    const saving = usePipelineStore
      .getState()
      .savePresets(usePipelineStore.getState().view!.def.presets);
    await useProjectsStore.getState().switchTo('frost-cathedral');
    await usePipelineStore.getState().load('frost-cathedral');
    expect(await saving).toBe(true);
    expect(usePipelineStore.getState().view?.project).toBe('frost-cathedral');
  });

  it('keeps sends that answer after a switch out of the new queue', async () => {
    const handle = await openMockWorkspace('configured', undefined, 'batch');
    mockIPC(async (cmd, args) => {
      if (cmd === 'provider_send_batch') await new Promise(r => setTimeout(r, 20));
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    await follow('frostveil');
    const providers = useProvidersStore.getState();
    await providers.openBatch('frostveil', ['lantern-post', 'sigil-door']);
    useProvidersStore.getState().setBatch({ provider: 'by-hand' });
    const sending = useProvidersStore.getState().sendBatch();
    await useProjectsStore.getState().switchTo('frost-cathedral');
    await follow('frost-cathedral');
    await sending;
    expect(useProvidersStore.getState().jobs.filter(j => j.project !== 'frost-cathedral')).toEqual(
      []
    );
  });

  it('does not switch to the project already open', async () => {
    await openMockWorkspace();
    expect(await useProjectsStore.getState().switchTo('frostveil')).toBe(false);
  });
});

describe('moving a lab direction', () => {
  it('takes the landing studio and its Refs card to another project', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('frostveil');
    useLabStore.getState().reuse('academy-landing');
    expect(await useLabStore.getState().moveDirection('academy-landing', 'frost-cathedral')).toBe(
      true
    );
    const lab = useLabStore.getState();
    expect(lab.view?.directions.some(d => d.id === 'academy-landing')).toBe(false);
    expect(lab.selected).not.toBe('academy-landing');
    expect(lab.draft.name).not.toBe('Academy landing');
    const projects = useAppStore.getState().workspace!.projects;
    const moon = projects.find(p => p.id === 'frostveil')!;
    const rooftop = projects.find(p => p.id === 'frost-cathedral')!;
    expect((moon.assets ?? []).some(a => a.name === 'Academy landing')).toBe(false);
    const card = (rooftop.assets ?? []).find(a => a.name === 'Academy landing');
    expect(card?.stage).toBe('refs');
    expect(card?.thumbnail).toMatch(/^refs\//);
    expect(useAppStore.getState().workspace?.project?.id).toBe('frostveil');
  });
});

describe('the workspace wide inbox', () => {
  const projects = [
    { id: 'frostveil', name: 'Frostveil' },
    { id: 'frost-cathedral', name: 'Frost cathedral' },
  ];
  const linked = {
    ...inboxItems('/w/inbox', 0)[0]!,
    link: {
      job: 'job-1',
      project: 'frostveil',
      asset: 'lantern',
      assetName: 'Lantern',
      provider: 'tripo' as const,
    },
  };

  it('accepts a sent model into the project it was sent for', () => {
    expect(acceptTarget(linked, projects, projects[1]!)?.id).toBe('frostveil');
    expect(acceptTarget({ ...linked, link: null }, projects, projects[1]!)?.id).toBe(
      'frost-cathedral'
    );
    const gone = { ...linked, link: { ...linked.link, project: 'deleted' } };
    expect(acceptTarget(gone, projects, projects[1]!)?.id).toBe('frost-cathedral');
  });

  it('names the project only when it is not the open one', () => {
    expect(sentFor(linked, projects, 'frostveil', 'Tripo')).toBe('Lantern, Tripo');
    expect(sentFor(linked, projects, 'frost-cathedral', 'Tripo')).toBe(
      'Lantern in Frostveil, Tripo'
    );
    expect(sentFor({ ...linked, link: null }, projects, 'frostveil', 'Tripo')).toBeNull();
  });
});
