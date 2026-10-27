import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useBoardPrefsStore, forgetStoredPrefs } from '@/state/use-board-prefs-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { BoardView } from '../BoardView';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('frostveil');
  useBoardPrefsStore.getState().update('frostveil', {
    layout: 'list',
    sort: { key: 'name', ascending: true },
  });
});

afterEach(() => {
  clearMocks();
  window.localStorage.clear();
  forgetStoredPrefs();
  useBoardPrefsStore.setState({ byProject: {} });
  useProvidersStore.setState({ project: null, jobs: [], batch: null, batchesSent: 0, busy: [] });
});

const grid = () => screen.getByRole('grid', { name: 'Assets' });
const names = () =>
  within(grid())
    .getAllByRole('row')
    .slice(1)
    .map(r => r.getAttribute('aria-label'));
const row = (name: string) => within(grid()).getByRole('row', { name });

async function renderList() {
  render(<BoardView queue={() => null} />);
  await screen.findByRole('grid', { name: 'Assets' });
}

describe('the Board list', () => {
  it('shows one row per asset with its numbers, sorted by a heading', async () => {
    await renderList();
    expect(names()[0]).toBe('Academy landing');
    const character = row('Elsheria');
    expect(character).toHaveTextContent('Elsheria');
    expect(character).toHaveTextContent('v2');
    expect(character).toHaveTextContent('115.7k');
    expect(character).toHaveTextContent('Needs review');

    await userEvent.click(screen.getByRole('button', { name: /^Tris/ }));
    expect(names()[0]).toBe('Veil platform');
    expect(screen.getByRole('columnheader', { name: /^Tris/ })).toHaveAttribute(
      'aria-sort',
      'descending'
    );
    await userEvent.click(screen.getByRole('button', { name: /^Tris/ }));
    expect(screen.getByRole('columnheader', { name: /^Tris/ })).toHaveAttribute(
      'aria-sort',
      'ascending'
    );
    expect(useBoardPrefsStore.getState().byProject['frostveil']?.sort).toEqual({
      key: 'tris',
      ascending: true,
    });
  });

  it('filters by stage and search, and remembers both', async () => {
    await renderList();
    const filter = screen.getByRole('group', { name: 'Stage filter' });
    await userEvent.click(within(filter).getByRole('button', { name: 'Blender' }));
    expect(names()).toEqual(['Crystal lantern', 'Elsheria', 'Orb shrine']);
    await userEvent.type(screen.getByRole('searchbox', { name: 'Search assets' }), 'lant');
    expect(names()).toEqual(['Crystal lantern']);
    expect(useBoardPrefsStore.getState().byProject['frostveil']).toMatchObject({
      stage: 'blender',
      query: 'lant',
    });
    await userEvent.click(within(filter).getByRole('button', { name: 'All' }));
    await userEvent.type(screen.getByRole('searchbox', { name: 'Search assets' }), 'zzz');
    expect(screen.getByText('No asset matches this stage and search.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Show every asset' }));
    expect(screen.getByRole('searchbox', { name: 'Search assets' })).toHaveValue('');
    expect(names().length).toBeGreaterThan(8);
  });

  it('selects rows with their boxes and Shift, then sends them as one batch', async () => {
    await renderList();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Elsheria' }));
    const user = userEvent.setup();
    await user.keyboard('{Shift>}');
    await user.click(screen.getByRole('checkbox', { name: 'Select Rune obelisk' }));
    await user.keyboard('{/Shift}');
    const bar = screen.getByRole('toolbar', { name: 'Selected assets' });
    const between = names().slice(names().indexOf('Elsheria'), names().indexOf('Rune obelisk') + 1);
    expect(bar).toHaveTextContent(`${between.length} assets selected`);
    expect(row('Elsheria')).toHaveAttribute('aria-selected', 'true');
    await userEvent.click(within(bar).getByRole('button', { name: 'Send to 3D' }));
    await waitFor(() =>
      expect(useProvidersStore.getState().batch?.draft?.items.map(i => i.name)).toEqual(between)
    );
  });

  it('moves with the arrows, toggles with Space, extends with Shift and opens the menu on Enter', async () => {
    await renderList();
    const first = row('Academy landing');
    expect(first).toHaveAttribute('tabindex', '0');
    first.focus();
    await userEvent.keyboard('{ArrowDown}');
    const second = names()[1]!;
    expect(row(second)).toHaveFocus();
    expect(row(second)).toHaveAttribute('tabindex', '0');
    expect(first).toHaveAttribute('tabindex', '-1');
    await userEvent.keyboard(' ');
    expect(row(second)).toHaveAttribute('aria-selected', 'true');
    await userEvent.keyboard('{Shift>}{ArrowDown}{/Shift}');
    expect(screen.getByRole('toolbar', { name: 'Selected assets' })).toHaveTextContent(
      '2 assets selected'
    );
    await userEvent.keyboard(' ');
    expect(screen.getByRole('toolbar', { name: 'Selected assets' })).toHaveTextContent(
      '1 asset selected'
    );
    await userEvent.keyboard('{Enter}');
    const third = names()[2]!;
    const menu = await screen.findByRole('menu', { name: `${third} actions` });
    expect(within(menu).getByRole('menuitem', { name: 'Send to 3D' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('menu', { name: `${third} actions` })).toBeNull();
  });

  it('has the card menu on every row', async () => {
    await renderList();
    await userEvent.click(screen.getByRole('button', { name: 'More for Lantern post' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    expect(screen.getByRole('dialog', { name: 'Rename asset' })).toBeInTheDocument();
  });
});
