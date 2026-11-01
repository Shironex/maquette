import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { NativeDropHandlers } from '@/lib/useNativeDrop';
import { useAppStore } from '@/state/use-app-store';
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
    drop(['/art/moon_gate.png', '/art/notes.txt']);
    await waitFor(() => expect(cards().map(a => a.name)).toContain('Moon gate'));
  });

  it('says what it takes when nothing dropped is an image', async () => {
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
    act(() => native.handlers.forEach(h => h.onDrop({ x: 1, y: 1 }, ['/art/notes.txt'])));
    expect(notice()).toMatchObject({ title: 'Drop a PNG, JPEG or WebP image' });
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
