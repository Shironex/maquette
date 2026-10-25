import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { forgetStoredPrefs, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import BoardHeader from './BoardHeader';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('moon-island');
});

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], queueOpen: false, busy: [] });
  window.localStorage.clear();
  forgetStoredPrefs();
  useBoardPrefsStore.setState({ byProject: {} });
});

describe('BoardHeader', () => {
  it('names the project and counts the By hand models still to make', () => {
    render(<BoardHeader queue={() => null} />);
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Moon Island');
    expect(screen.getByRole('button', { name: /By hand/ })).toHaveTextContent('By hand5');
  });

  it('opens the 3D queue the shell supplies', async () => {
    const queue = vi.fn(() => <div role="dialog" aria-label="3D queue panel" />);
    render(<BoardHeader queue={queue} />);
    expect(queue).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: /3D queue/ }));
    expect(screen.getByRole('dialog', { name: '3D queue panel' })).toBeInTheDocument();
    expect(useProvidersStore.getState().queueOpen).toBe(true);
  });

  it('toggles the add asset form', async () => {
    render(<BoardHeader queue={() => null} />);
    const add = screen.getByRole('button', { name: 'Add asset' });
    await userEvent.click(add);
    expect(screen.getByRole('form', { name: 'Add asset' })).toBeInTheDocument();
    expect(add).toHaveAttribute('aria-expanded', 'true');
    await userEvent.click(add);
    expect(screen.queryByRole('form', { name: 'Add asset' })).toBeNull();
  });

  it('switches the Board to the list and remembers it for the project', async () => {
    render(<BoardHeader queue={() => null} />);
    const layout = screen.getByRole('group', { name: 'Board layout' });
    const list = within(layout).getByRole('button', { name: 'List' });
    expect(within(layout).getByRole('button', { name: 'Board' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await userEvent.click(list);
    expect(list).toHaveAttribute('aria-pressed', 'true');
    expect(window.localStorage.getItem('maquette.board.moon-island')).toContain('"list"');
  });
});
