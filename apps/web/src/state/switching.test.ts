import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { inboxItems } from '@/ipc/mock/inbox-fixtures';
import { useAppStore } from './app-store';
import { acceptTarget, sentFor } from './inbox';
import { useLabStore } from './lab-store';
import { usePipelineStore } from './pipeline-store';
import { useProjectsStore } from './projects-store';
import { useProvidersStore } from './providers-store';

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
    await follow('moon-island');
    expect(usePipelineStore.getState().view?.runs.length).toBeGreaterThan(0);
    const lab = useLabStore.getState();
    expect(lab.selected).not.toBeNull();
    expect(lab.draft.name).not.toBe('');
    useLabStore.setState({ lightbox: '/somewhere/concept.png' });
    await useProvidersStore.getState().openPanel('moon-island', 'maquette-landing-studio');
    expect(useProvidersStore.getState().panel?.project).toBe('moon-island');

    expect(await useProjectsStore.getState().switchTo('rooftop-cinema')).toBe(true);
    await follow('rooftop-cinema');

    expect(useAppStore.getState().workspace?.project?.id).toBe('rooftop-cinema');
    const pipeline = usePipelineStore.getState().view;
    expect(pipeline?.project).toBe('rooftop-cinema');
    expect(pipeline?.runs).toEqual([]);
    expect(pipeline?.active).toBeNull();
    const after = useLabStore.getState();
    expect(after.view?.project).toBe('rooftop-cinema');
    expect(after.lightbox).toBeNull();
    expect(after.view?.directions.some(d => d.id === lab.selected)).toBe(false);
    expect(useProvidersStore.getState().project).toBe('rooftop-cinema');
    expect(useProvidersStore.getState().panel).toBeNull();
  });

  it('closes the batch sheet and the downloads of the project left', async () => {
    await openMockWorkspace('configured', undefined, 'batch');
    await follow('moon-island');
    await useProvidersStore.getState().openBatch('moon-island', ['desk-lamp', 'brush-cup']);
    await useProvidersStore.getState().checkDownloads();
    expect(useProvidersStore.getState().batch?.project).toBe('moon-island');
    expect(useProvidersStore.getState().downloads?.project).toBe('moon-island');

    await useProjectsStore.getState().switchTo('rooftop-cinema');
    await follow('rooftop-cinema');
    expect(useProvidersStore.getState().batch).toBeNull();
    expect(useProvidersStore.getState().downloads).toBeNull();
  });

  it('drops an answer for the project it just left', async () => {
    await openMockWorkspace('running');
    const slow = usePipelineStore.getState().load('moon-island');
    const lab = useLabStore.getState().load('moon-island');
    await useProjectsStore.getState().switchTo('rooftop-cinema');
    await follow('rooftop-cinema');
    await Promise.all([slow, lab]);
    expect(usePipelineStore.getState().view?.project).toBe('rooftop-cinema');
    expect(useLabStore.getState().view?.project).toBe('rooftop-cinema');
  });

  it('keeps a presets save for the project left out of the open one', async () => {
    const handle = await openMockWorkspace();
    // The save answers after the switch landed.
    mockIPC(async (cmd, args) => {
      if (cmd === 'pipeline_save_presets') await new Promise(r => setTimeout(r, 20));
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    await usePipelineStore.getState().load('moon-island');
    const saving = usePipelineStore
      .getState()
      .savePresets(usePipelineStore.getState().view!.def.presets);
    await useProjectsStore.getState().switchTo('rooftop-cinema');
    await usePipelineStore.getState().load('rooftop-cinema');
    expect(await saving).toBe(true);
    expect(usePipelineStore.getState().view?.project).toBe('rooftop-cinema');
  });

  it('keeps sends that answer after a switch out of the new queue', async () => {
    const handle = await openMockWorkspace('configured', undefined, 'batch');
    mockIPC(async (cmd, args) => {
      if (cmd === 'provider_send_batch') await new Promise(r => setTimeout(r, 20));
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    await follow('moon-island');
    const providers = useProvidersStore.getState();
    await providers.openBatch('moon-island', ['desk-lamp', 'brush-cup']);
    useProvidersStore.getState().setBatch({ provider: 'by-hand' });
    const sending = useProvidersStore.getState().sendBatch();
    await useProjectsStore.getState().switchTo('rooftop-cinema');
    await follow('rooftop-cinema');
    await sending;
    expect(useProvidersStore.getState().jobs.filter(j => j.project !== 'rooftop-cinema')).toEqual(
      []
    );
  });

  it('does not switch to the project already open', async () => {
    await openMockWorkspace();
    expect(await useProjectsStore.getState().switchTo('moon-island')).toBe(false);
  });
});

describe('moving a lab direction', () => {
  it('takes the landing studio and its Refs card to another project', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('moon-island');
    useLabStore.getState().reuse('maquette-landing-studio');
    expect(
      await useLabStore.getState().moveDirection('maquette-landing-studio', 'rooftop-cinema')
    ).toBe(true);
    const lab = useLabStore.getState();
    expect(lab.view?.directions.some(d => d.id === 'maquette-landing-studio')).toBe(false);
    expect(lab.selected).not.toBe('maquette-landing-studio');
    expect(lab.draft.name).not.toBe('Maquette landing studio');
    const projects = useAppStore.getState().workspace!.projects;
    const moon = projects.find(p => p.id === 'moon-island')!;
    const rooftop = projects.find(p => p.id === 'rooftop-cinema')!;
    expect((moon.assets ?? []).some(a => a.name === 'Maquette landing studio')).toBe(false);
    const card = (rooftop.assets ?? []).find(a => a.name === 'Maquette landing studio');
    expect(card?.stage).toBe('refs');
    expect(card?.thumbnail).toMatch(/^refs\//);
    expect(useAppStore.getState().workspace?.project?.id).toBe('moon-island');
  });
});

describe('the workspace wide inbox', () => {
  const projects = [
    { id: 'moon-island', name: 'Moon Island' },
    { id: 'rooftop-cinema', name: 'Rooftop cinema' },
  ];
  const linked = {
    ...inboxItems('/w/inbox', 0)[0]!,
    link: {
      job: 'job-1',
      project: 'moon-island',
      asset: 'lantern',
      assetName: 'Lantern',
      provider: 'tripo' as const,
    },
  };

  it('accepts a sent model into the project it was sent for', () => {
    expect(acceptTarget(linked, projects, projects[1]!)?.id).toBe('moon-island');
    expect(acceptTarget({ ...linked, link: null }, projects, projects[1]!)?.id).toBe(
      'rooftop-cinema'
    );
    const gone = { ...linked, link: { ...linked.link, project: 'deleted' } };
    expect(acceptTarget(gone, projects, projects[1]!)?.id).toBe('rooftop-cinema');
  });

  it('names the project only when it is not the open one', () => {
    expect(sentFor(linked, projects, 'moon-island', 'Tripo')).toBe('Lantern, Tripo');
    expect(sentFor(linked, projects, 'rooftop-cinema', 'Tripo')).toBe(
      'Lantern in Moon Island, Tripo'
    );
    expect(sentFor({ ...linked, link: null }, projects, 'moon-island', 'Tripo')).toBeNull();
  });
});
