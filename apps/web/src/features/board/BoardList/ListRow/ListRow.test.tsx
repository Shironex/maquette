import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clock } from '@/lib/clock';
import { listRow } from '@/state/board-list';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { fakeBoardEnv, mockAsset } from '../../fakeBoardEnv';
import ListRow from './ListRow';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('moon-island');
});

afterEach(() => {
  clearMocks();
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
    const { board } = renderRow('Character');
    const row = screen.getByRole('row', { name: 'Character' });
    expect(row).toHaveTextContent('Blender');
    expect(row).toHaveTextContent('1.4 MB');
    expect(row).toHaveTextContent('source');
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Character' }));
    expect(board.selection.check).toHaveBeenCalledWith('character', true, false);
  });

  it('opens its menu from the dots', async () => {
    const { onToggleMenu } = renderRow('Character');
    await userEvent.click(screen.getByRole('button', { name: 'More for Character' }));
    expect(onToggleMenu).toHaveBeenCalledWith('character');
  });

  it('shows the card menu while it is open, and a waiting send by hand', () => {
    const board = fakeBoardEnv();
    renderRow('Brush cup', { ...board, actions: { ...board.actions, menuFor: 'brush-cup' } });
    expect(screen.getByRole('row', { name: 'Brush cup' })).toHaveTextContent(
      'Waiting for your model'
    );
    expect(screen.getByRole('menuitem', { name: 'Add model...' })).toBeInTheDocument();
  });
});
