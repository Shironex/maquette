import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';
import { useProjectsStore } from '@/state/use-projects-store';
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
    await useLabStore.getState().load('frostveil');
    useLabStore.setState({ selected: 'frozen-lake-arena' });
    expect(picked('frozen-lake-arena')).toHaveLength(1);

    const id = await useLabStore.getState().sendPicks('prop', true);
    expect(id).toBeTruthy();
    expect(picked('frozen-lake-arena')).toEqual([]);
    // The send is remembered on the image, not by the mark.
    const sent = useLabStore
      .getState()
      .view?.directions.find(d => d.id === 'frozen-lake-arena')
      ?.images.filter(i => i.refFile);
    expect(sent).toHaveLength(1);
  });

  it('keep their marks when the send failed', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('frostveil');
    useLabStore.setState({ selected: 'veil-gate-kit' });
    // Nothing is picked there, so the send is refused.
    expect(await useLabStore.getState().sendPicks('prop', true)).toBeNull();
    useLabStore.setState({ selected: 'frozen-lake-arena' });
    expect(picked('frozen-lake-arena')).toHaveLength(1);
  });

  it('drop the marks of an objects direction after its send', async () => {
    await openMockWorkspace('configured', undefined, 'batch');
    await useLabStore.getState().load('frostveil');
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
    await useLabStore.getState().load('frostveil');
    useLabStore.setState({ selected: 'frozen-lake-arena' });
    const sending = useLabStore.getState().sendPicks('prop', true);
    useLabStore.getState().select('academy-landing');
    expect(await sending).toBeTruthy();
    expect(picked('frozen-lake-arena')).toEqual([]);
    expect(picked('academy-landing')).toHaveLength(1);
  });
});

describe('Make object refs and a project switch', () => {
  const source = {
    project: 'frostveil',
    source: 'concepts/frozen-lake-arena/frozen-lake-arena_01.png',
    sourcePath: '/x/frozen-lake-arena_01.png',
    sourceName: 'Frozen lake arena',
  };

  it('closes with the project it was opened in', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('frostveil');
    useLabStore.getState().openObjects(source);
    useLabStore.getState().setObjects({ objects: ['Bench'] });
    expect(await useProjectsStore.getState().switchTo('frost-cathedral')).toBe(true);
    await useLabStore.getState().load('frost-cathedral');
    expect(useLabStore.getState().objects).toBeNull();
  });

  it('keeps the lab on the open project when the answer is for another', async () => {
    await openMockWorkspace();
    await useProjectsStore.getState().switchTo('frost-cathedral');
    await useLabStore.getState().load('frost-cathedral');
    expect(useAppStore.getState().workspace?.project?.id).toBe('frost-cathedral');
    // Opened for Frostveil just before the switch landed.
    useLabStore.setState({
      objects: {
        ...source,
        objects: ['Bench'],
        notes: {},
        aspect: '1:1',
        kind: 'prop',
        useStyle: true,
      },
    });
    expect(await useLabStore.getState().makeObjects()).toBe(true);
    expect(useLabStore.getState().view?.project).toBe('frost-cathedral');
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
    await useLabStore.getState().load('frostveil');
    const adding = useLabStore.getState().addRefs(['/Users/me/Desktop/porch-sketch.png']);
    await useProjectsStore.getState().switchTo('frost-cathedral');
    await useLabStore.getState().load('frost-cathedral');
    const refs = useLabStore.getState().draft.refs;
    await adding;
    expect(useLabStore.getState().view?.project).toBe('frost-cathedral');
    expect(useLabStore.getState().draft.refs).toEqual(refs);
  });
});

describe('the project style', () => {
  it('goes along with a new direction unless its brief turns it off', async () => {
    const { openMockStyle } = await import('@/test/mock-ipc');
    await openMockStyle();
    await useLabStore.getState().load('frostveil');
    const lab = useLabStore.getState();
    lab.setDraft({ name: 'Rune bridge', prompt: 'A rune bridge over the frozen lake', count: 1 });
    await lab.generate();
    const styled = useLabStore.getState().view!;
    expect(styled.directions.find(d => d.id === 'rune-bridge')).toMatchObject({
      styleOff: false,
      styleRevision: 3,
    });
    expect(styled.jobs.find(j => j.direction === 'rune-bridge')?.styleRevision).toBe(3);

    lab.setDraft({ name: 'Ice throne', prompt: 'An ice throne', useStyle: false });
    await useLabStore.getState().generate();
    const off = useLabStore.getState().view!;
    expect(off.directions.find(d => d.id === 'ice-throne')).toMatchObject({
      styleOff: true,
      styleRevision: null,
    });
    // Reusing a brief keeps its switch.
    useLabStore.getState().reuse('elsheria-sheets');
    expect(useLabStore.getState().draft.useStyle).toBe(false);
    useLabStore.getState().reuse('frozen-lake-arena');
    expect(useLabStore.getState().draft.useStyle).toBe(true);
  });

  it('takes a concept straight from the grid, once', async () => {
    const { openMockStyle } = await import('@/test/mock-ipc');
    await openMockStyle();
    await useLabStore.getState().load('frostveil');
    const file = 'concepts/frozen-lake-arena/frozen-lake-arena_02.png';
    await useLabStore.getState().addToStyle(file);
    const board = useLabStore.getState().view!.style;
    expect(board.revision).toBe(4);
    expect(board.images.at(-1)).toEqual({ file: 'style/frozen_lake_arena_02.png', source: file });
    await useLabStore.getState().addToStyle(file);
    expect(useLabStore.getState().view!.style.revision).toBe(4);
  });
});
