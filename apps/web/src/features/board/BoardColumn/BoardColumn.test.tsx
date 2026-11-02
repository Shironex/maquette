import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { NativeDropHandlers } from '@/lib/useNativeDrop';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { DRAG_TYPE } from '../constants';
import { fakeBoardEnv, mockAsset } from '../fakeBoardEnv';
import BoardColumn from './BoardColumn';

const native = vi.hoisted(() => ({ handlers: [] as NativeDropHandlers[] }));
vi.mock('@/lib/useNativeDrop', () => ({
  useNativeDrop: (handlers: NativeDropHandlers) => native.handlers.push(handlers),
}));

const cards = () => useAppStore.getState().workspace?.project?.assets ?? [];
const notice = () => useAppStore.getState().notice;

beforeEach(async () => {
  native.handlers = [];
  await openMockWorkspace('configured', undefined, 'batch');
});

afterEach(() => clearMocks());

function dragData(id: string) {
  return { types: [DRAG_TYPE], getData: (type: string) => (type === DRAG_TYPE ? id : '') };
}

describe('BoardColumn', () => {
  it('names the stage with its count and lists its cards', () => {
    render(
      <BoardColumn
        stage="blender"
        assets={[mockAsset('Elsheria'), mockAsset('Crystal lantern')]}
        boardEmpty={false}
        board={fakeBoardEnv()}
      />
    );
    const column = screen.getByRole('region', { name: 'Blender, 2 assets' });
    expect(column).toHaveTextContent('Elsheria');
    expect(column).toHaveTextContent('Crystal lantern');
    expect(screen.queryByText('Drag an asset here')).toBeNull();
  });

  it('renders only the cards in view of a long column, and the next ones as it scrolls', () => {
    const one = mockAsset('Elsheria');
    const assets = Array.from({ length: 200 }, (_, i) => ({
      ...one,
      id: `prop-${i}`,
      name: `Prop ${i}`,
    }));
    render(
      <BoardColumn stage="blender" assets={assets} boardEmpty={false} board={fakeBoardEnv()} />
    );
    const column = screen.getByRole('region', { name: 'Blender, 200 assets' });
    expect(screen.getByText('Prop 0')).toBeInTheDocument();
    expect(screen.queryByText('Prop 199')).toBeNull();
    act(() => {
      column.scrollTop = 150 * 220;
      fireEvent.scroll(column);
    });
    expect(screen.getByText('Prop 150')).toBeInTheDocument();
    expect(screen.queryByText('Prop 0')).toBeNull();
  });

  it('shows the concept drop zone on an empty Board, and a drag target elsewhere', async () => {
    const board = fakeBoardEnv();
    const { rerender } = render(
      <BoardColumn stage="concept" assets={[]} boardEmpty board={board} />
    );
    expect(screen.getByRole('button', { name: /Drop concept art/ })).toBeVisible();
    expect(screen.queryByText('Drag an asset here')).toBeNull();
    rerender(<BoardColumn stage="refs" assets={[]} boardEmpty board={board} />);
    expect(screen.getByText('Drag an asset here')).toBeInTheDocument();
  });

  it('imports the images chosen from the empty drop zone', async () => {
    render(<BoardColumn stage="concept" assets={[]} boardEmpty board={fakeBoardEnv()} />);
    await userEvent.click(screen.getByRole('button', { name: /Drop concept art/ }));
    await waitFor(() => expect(cards().some(a => a.stage === 'concept' && a.thumbnail)).toBe(true));
    expect(notice()).toMatchObject({ status: 'done', action: { label: 'Open Lab' } });
  });

  it('takes images dropped on a column that already has cards', async () => {
    render(
      <BoardColumn
        stage="concept"
        assets={[mockAsset('Elsheria')]}
        boardEmpty={false}
        board={fakeBoardEnv()}
      />
    );
    const file = new File(['x'], 'stone_lantern.png', { type: 'image/png' });
    fireEvent.drop(screen.getByRole('region', { name: /^Concept/ }), {
      dataTransfer: { types: ['Files'], files: [file], getData: () => '' },
    });
    await waitFor(() => expect(cards().map(a => a.name)).toContain('Stone lantern'));
  });

  it('takes a native drop over the Concept column and leaves other columns alone', async () => {
    render(
      <>
        <BoardColumn
          stage="concept"
          assets={[mockAsset('Elsheria')]}
          boardEmpty={false}
          board={fakeBoardEnv()}
        />
        <BoardColumn stage="refs" assets={[]} boardEmpty={false} board={fakeBoardEnv()} />
      </>
    );
    const concept = screen.getByRole('region', { name: /^Concept/ });
    const refs = screen.getByRole('region', { name: /^Refs/ });
    const spot = vi.fn<() => Element | null>(() => refs);
    Object.defineProperty(document, 'elementFromPoint', { configurable: true, value: spot });
    const drop = (paths: string[]) =>
      act(() => native.handlers.forEach(h => h.onDrop({ x: 1, y: 1 }, paths)));
    drop(['/art/over_refs.png']);
    expect(cards().map(a => a.name)).not.toContain('Over refs');
    spot.mockReturnValue(concept);
    drop(['/art/moon_gate.png']);
    await waitFor(() => expect(cards().map(a => a.name)).toContain('Moon gate'));
  });

  const nativeDropOnConcept = (paths: string[]) => {
    render(
      <BoardColumn
        stage="concept"
        assets={[mockAsset('Elsheria')]}
        boardEmpty={false}
        board={fakeBoardEnv()}
      />
    );
    const concept = screen.getByRole('region', { name: /^Concept/ });
    Object.defineProperty(document, 'elementFromPoint', {
      configurable: true,
      value: () => concept,
    });
    act(() => native.handlers.forEach(h => h.onDrop({ x: 1, y: 1 }, paths)));
  };

  it('stops the whole import and names the file when one is not an image', async () => {
    nativeDropOnConcept(['/art/moon_gate.png', '/art/layers.psd']);
    await waitFor(() => expect(notice()?.status).toBe('failed'));
    expect(notice()?.detail).toContain('layers.psd');
    expect(cards().map(a => a.name)).not.toContain('Moon gate');
  });

  it('lets the core judge a file with no extension', async () => {
    nativeDropOnConcept(['/art/moon_gate']);
    await waitFor(() => expect(cards().map(a => a.name)).toContain('Moon gate'));
  });

  it('stops a browser drop the same way, with the file named', async () => {
    render(
      <BoardColumn
        stage="concept"
        assets={[mockAsset('Elsheria')]}
        boardEmpty={false}
        board={fakeBoardEnv()}
      />
    );
    const good = new File(['x'], 'moon_gate.png', { type: 'image/png' });
    const bad = new File(['x'], 'layers.psd');
    fireEvent.drop(screen.getByRole('region', { name: /^Concept/ }), {
      dataTransfer: { types: ['Files'], files: [good, bad], getData: () => '' },
    });
    await waitFor(() => expect(notice()?.detail).toContain('layers.psd'));
    expect(cards().map(a => a.name)).not.toContain('Moon gate');
  });

  it('shows the new direction in the Lab right after an import', async () => {
    await useLabStore.getState().load('frostveil');
    nativeDropOnConcept(['/art/moon_gate.png']);
    await waitFor(() =>
      expect(useLabStore.getState().view?.directions.map(d => d.name)).toContain('Moon gate')
    );
  });

  const answerFor = (name: string) => ({
    direction: { name },
    added: ['concepts/a.png'],
    asset: {},
    project: { ...useAppStore.getState().workspace!.project!, name },
  });

  it('reads the project once after the last import, whichever answers first', async () => {
    const project = useAppStore.getState().workspace!.project!;
    let finishBig!: (value: unknown) => void;
    const big = new Promise(resolve => {
      finishBig = resolve;
    });
    const imports = vi
      .spyOn(commands, 'labImportConcepts')
      .mockReturnValueOnce(big as never)
      .mockResolvedValueOnce(answerFor('Small') as never);
    const read = vi
      .spyOn(commands, 'projectGet')
      .mockResolvedValue({ ...project, name: 'Both' } as never);
    nativeDropOnConcept(['/art/big.png']);
    act(() => native.handlers.forEach(h => h.onDrop({ x: 1, y: 1 }, ['/art/small.png'])));
    await waitFor(() => expect(imports).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(notice()?.title).toContain('Small'));
    expect(read).not.toHaveBeenCalled();
    finishBig(answerFor('Big'));
    await waitFor(() => expect(useAppStore.getState().workspace?.project?.name).toBe('Both'));
    expect(read).toHaveBeenCalledTimes(1);
    imports.mockRestore();
    read.mockRestore();
  });

  it('reads the project after a failed import too', async () => {
    const project = useAppStore.getState().workspace!.project!;
    const read = vi
      .spyOn(commands, 'projectGet')
      .mockResolvedValue({ ...project, name: 'Read' } as never);
    nativeDropOnConcept(['/art/layers.psd']);
    await waitFor(() => expect(useAppStore.getState().workspace?.project?.name).toBe('Read'));
    read.mockRestore();
  });

  it('leaves the project now open alone when an import answers late', async () => {
    await useLabStore.getState().load('frostveil');
    let finish!: (value: unknown) => void;
    const late = new Promise(resolve => {
      finish = resolve;
    });
    const imports = vi.spyOn(commands, 'labImportConcepts').mockReturnValueOnce(late as never);
    const read = vi.spyOn(commands, 'projectGet');
    nativeDropOnConcept(['/art/one.png']);
    await waitFor(() => expect(imports).toHaveBeenCalled());
    await useProjectsStore.getState().switchTo('frost-cathedral');
    await useLabStore.getState().load('frost-cathedral');
    useLabStore.getState().setDraft({ prompt: 'Still typing' });
    finish(answerFor('Late'));
    await act(async () => {
      await late;
      await new Promise(resolve => setTimeout(resolve, 20));
    });
    expect(useLabStore.getState().view?.project).toBe('frost-cathedral');
    expect(useLabStore.getState().draft.prompt).toBe('Still typing');
    expect(useAppStore.getState().workspace?.project?.id).toBe('frost-cathedral');
    expect(read).not.toHaveBeenCalled();
    imports.mockRestore();
    read.mockRestore();
  });

  it('leaves a native drop on a By hand card to that card', () => {
    render(
      <BoardColumn
        stage="concept"
        assets={[mockAsset('Elsheria')]}
        boardEmpty={false}
        board={fakeBoardEnv()}
      />
    );
    const card = document.createElement('div');
    card.dataset.dropJob = 'job-1';
    screen.getByRole('region', { name: /^Concept/ }).append(card);
    Object.defineProperty(document, 'elementFromPoint', { configurable: true, value: () => card });
    act(() => native.handlers.forEach(h => h.onDrop({ x: 1, y: 1 }, ['/art/moon_gate.png'])));
    expect(cards().map(a => a.name)).not.toContain('Moon gate');
  });

  it('lights up under a dragged card and moves it on drop', () => {
    const board = fakeBoardEnv();
    render(<BoardColumn stage="web" assets={[]} boardEmpty={false} board={board} />);
    const column = screen.getByRole('region', { name: 'Web, 0 assets' });
    fireEvent.dragOver(column, { dataTransfer: dragData('elsheria') });
    expect(column).toHaveClass('border-clay-line');
    fireEvent.drop(column, { dataTransfer: dragData('elsheria') });
    expect(column).not.toHaveClass('border-clay-line');
    expect(board.actions.dropOn).toHaveBeenCalledWith('web', 'elsheria');
  });
});
