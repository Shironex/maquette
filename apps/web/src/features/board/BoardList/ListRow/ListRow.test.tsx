import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clock } from '@/lib/clock';
import { listRow } from '@/state/board-list';
import { useGameStore } from '@/state/use-game-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { fakeBoardEnv, mockAsset } from '../../fakeBoardEnv';
import ListRow from './ListRow';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('frostveil');
});

afterEach(() => {
  clearMocks();
  useGameStore.setState({ rendering: false, waitingFor: null });
  useProvidersStore.setState({ project: null, jobs: [], batch: null, batchesSent: 0, busy: [] });
});

function renderRow(name: string, board = fakeBoardEnv()) {
  const onToggleMenu = vi.fn();
  render(
    <table>
      <tbody>
        <ListRow
          row={listRow(mockAsset(name), [])}
          board={board}
          tabStop
          menuUp={false}
          onToggleMenu={onToggleMenu}
          onFocus={vi.fn()}
          nowMs={clock.nowMs()}
        />
      </tbody>
    </table>
  );
  return { board, onToggleMenu };
}

describe('ListRow', () => {
  it('shows the asset and checks its box, with Shift for a range', async () => {
    const { board } = renderRow('Elsheria');
    const row = screen.getByRole('row', { name: 'Elsheria' });
    expect(row).toHaveTextContent('Blender');
    expect(row).toHaveTextContent('1.1 MB');
    expect(row).toHaveTextContent('source');
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Elsheria' }));
    expect(board.selection.check).toHaveBeenCalledWith('elsheria', true, false);
  });

  it('tells a screen reader while its preview renders', () => {
    useGameStore.setState({ rendering: true, queued: [], waitingFor: 'frostveil' });
    const asset = mockAsset('Elsheria');
    render(
      <table>
        <tbody>
          <ListRow
            row={listRow({ ...asset, thumbnail: null }, [])}
            board={fakeBoardEnv()}
            tabStop
            menuUp={false}
            onToggleMenu={vi.fn()}
            onFocus={vi.fn()}
            nowMs={clock.nowMs()}
          />
        </tbody>
      </table>
    );
    expect(screen.getByRole('row')).toHaveTextContent('Rendering a preview');
  });

  it('opens its menu from the dots', async () => {
    const { onToggleMenu } = renderRow('Elsheria');
    await userEvent.click(screen.getByRole('button', { name: 'More for Elsheria' }));
    expect(onToggleMenu).toHaveBeenCalledWith('elsheria');
  });

  it('shows the card menu while it is open, and a waiting send by hand', () => {
    const board = fakeBoardEnv();
    renderRow('Sigil door', { ...board, actions: { ...board.actions, menuFor: 'sigil-door' } });
    expect(screen.getByRole('row', { name: 'Sigil door' })).toHaveTextContent(
      'Waiting for your model'
    );
    expect(screen.getByRole('menuitem', { name: 'Add model...' })).toBeInTheDocument();
  });
});
