import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { DRAG_TYPE } from '../constants';
import { fakeBoardEnv, mockAsset } from '../fakeBoardEnv';
import BoardColumn from './BoardColumn';

beforeEach(async () => {
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
    await userEvent.click(screen.getByRole('button', { name: /Drop concept art/ }));
    expect(board.actions.later).toHaveBeenCalledWith('Concept import');
    expect(screen.queryByText('Drag an asset here')).toBeNull();
    rerender(<BoardColumn stage="refs" assets={[]} boardEmpty board={board} />);
    expect(screen.getByText('Drag an asset here')).toBeInTheDocument();
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
