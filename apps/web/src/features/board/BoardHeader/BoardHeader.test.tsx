import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { forgetStoredPrefs, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockGame, openMockWorkspace } from '@/test/mock-ipc';
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
  it('gives every button of the header a tooltip', async () => {
    await openMockGame('crypt');
    render(<BoardHeader queue={() => null} />);
    const buttons = screen.getAllByRole('button');
    const names = buttons.map(b => b.textContent ?? '');
    for (const name of ['By hand', '3D queue', 'Import models', 'Built asset', 'Add asset'])
      expect(
        names.some(n => n.startsWith(name)),
        name
      ).toBe(true);
    for (const button of buttons) {
      await userEvent.hover(button);
      expect(await screen.findByRole('tooltip'), button.textContent ?? '').toBeVisible();
      await userEvent.unhover(button);
      await waitFor(() => expect(screen.queryByRole('tooltip')).toBeNull());
    }
  });

  it('says what By hand opens', async () => {
    render(<BoardHeader queue={() => null} />);
    await userEvent.hover(screen.getByRole('button', { name: /By hand/ }));
    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      "By hand: models waiting for you to make on the provider's site"
    );
  });

  it('renders a preview for each imported game model without one', async () => {
    await openMockGame('crypt');
    const without = () =>
      useAppStore
        .getState()
        .workspace!.project!.assets!.filter(a => !a.thumbnail?.startsWith('thumbs/'));
    expect(without().length).toBeGreaterThan(0);
    render(<BoardHeader queue={() => null} />);
    await userEvent.click(screen.getByRole('button', { name: /^Render \d+ previews?$/ }));
    await vi.waitFor(() => expect(without()).toEqual([]));
    expect(screen.queryByRole('button', { name: /^Render \d+ previews?$/ })).toBeNull();
  });

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
