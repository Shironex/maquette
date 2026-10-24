import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/app-store';
import { useLabStore } from '@/state/lab-store';
import { useProjectsStore } from '@/state/projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';

afterEach(() => {
  clearMocks();
  useLabStore.setState({ view: null, selected: null, objects: null, busy: null });
});

const picked = (id: string) =>
  useLabStore
    .getState()
    .view?.directions.find(d => d.id === id)
    ?.images.filter(i => i.picked)
    .map(i => i.file) ?? [];

describe('the picks after a send', () => {
  it('drop their marks once they reached Refs', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('moon-island');
    useLabStore.setState({ selected: 'moon-island-porch' });
    expect(picked('moon-island-porch')).toHaveLength(1);

    const id = await useLabStore.getState().sendPicks('prop', true);
    expect(id).toBeTruthy();
    expect(picked('moon-island-porch')).toEqual([]);
    // The send is remembered on the image, not by the mark.
    const sent = useLabStore
      .getState()
      .view?.directions.find(d => d.id === 'moon-island-porch')
      ?.images.filter(i => i.refFile);
    expect(sent).toHaveLength(1);
  });

  it('keep their marks when the send failed', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('moon-island');
    useLabStore.setState({ selected: 'late-train' });
    // Nothing is picked there, so the send is refused.
    expect(await useLabStore.getState().sendPicks('prop', true)).toBeNull();
    useLabStore.setState({ selected: 'moon-island-porch' });
    expect(picked('moon-island-porch')).toHaveLength(1);
  });

  it('drop the marks of an objects direction after its send', async () => {
    await openMockWorkspace('configured', undefined, 'batch');
    await useLabStore.getState().load('moon-island');
    const objects = useLabStore.getState().view!.directions.find(d => d.source);
    expect(objects).toBeTruthy();
    useLabStore.setState({ selected: objects!.id });
    const before = picked(objects!.id).length;
    expect(before).toBeGreaterThan(0);
    const ids = await useLabStore.getState().sendObjectPicks(true);
    expect(ids?.length).toBeGreaterThan(0);
    expect(picked(objects!.id)).toEqual([]);
  });

  it('leave the picks of a direction chosen while the send ran', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('moon-island');
    useLabStore.setState({ selected: 'moon-island-porch' });
    const sending = useLabStore.getState().sendPicks('prop', true);
    useLabStore.getState().select('maquette-landing-studio');
    expect(await sending).toBeTruthy();
    expect(picked('moon-island-porch')).toEqual([]);
    expect(picked('maquette-landing-studio')).toHaveLength(1);
  });
});

describe('Make object refs and a project switch', () => {
  const source = {
    project: 'moon-island',
    source: 'concepts/moon-island-porch/moon-island-porch_01.png',
    sourcePath: '/x/moon-island-porch_01.png',
    sourceName: 'Moon island porch',
  };

  it('closes with the project it was opened in', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('moon-island');
    useLabStore.getState().openObjects(source);
    useLabStore.getState().setObjects({ objects: ['Bench'] });
    expect(await useProjectsStore.getState().switchTo('rooftop-cinema')).toBe(true);
    await useLabStore.getState().load('rooftop-cinema');
    expect(useLabStore.getState().objects).toBeNull();
  });

  it('keeps the lab on the open project when the answer is for another', async () => {
    await openMockWorkspace();
    await useProjectsStore.getState().switchTo('rooftop-cinema');
    await useLabStore.getState().load('rooftop-cinema');
    expect(useAppStore.getState().workspace?.project?.id).toBe('rooftop-cinema');
    // Opened for Moon Island just before the switch landed.
    useLabStore.setState({
      objects: { ...source, objects: ['Bench'], notes: {}, aspect: '1:1', kind: 'prop' },
    });
    expect(await useLabStore.getState().makeObjects()).toBe(true);
    expect(useLabStore.getState().view?.project).toBe('rooftop-cinema');
    expect(useLabStore.getState().objects).toBeNull();
  });
});

describe('lab answers after a project switch', () => {
  it('keep refs copied for the project left out of the new composer', async () => {
    const handle = await openMockWorkspace();
    mockIPC(async (cmd, args) => {
      if (cmd === 'lab_import_refs') await new Promise(r => setTimeout(r, 20));
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    await useLabStore.getState().load('moon-island');
    const adding = useLabStore.getState().addRefs(['/Users/me/Desktop/porch-sketch.png']);
    await useProjectsStore.getState().switchTo('rooftop-cinema');
    await useLabStore.getState().load('rooftop-cinema');
    const refs = useLabStore.getState().draft.refs;
    await adding;
    expect(useLabStore.getState().view?.project).toBe('rooftop-cinema');
    expect(useLabStore.getState().draft.refs).toEqual(refs);
  });
});
