import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { NO_FILTERS } from '@/state/library';
import { useAppStore } from '@/state/use-app-store';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryView from './LibraryView';

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({
    view: null,
    filters: NO_FILTERS,
    selected: null,
    adding: null,
    layout: 'grid',
  });
});

describe('the Library view', () => {
  it('renders only the models in view of a long library, in the grid and the list', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    const models = await screen.findByRole('region', { name: 'Models' });
    await within(models).findByText('8 models · most used first');
    const view = useLibraryStore.getState().view!;
    const one = view.items[0]!;
    const items = Array.from({ length: 300 }, (_, i) => ({
      ...one,
      entry: { ...one.entry, id: `copy-${i}`, name: `Copy ${i}` },
    }));
    act(() => useLibraryStore.setState({ view: { ...view, items } }));
    expect(within(models).getByText('300 models · most used first')).toBeVisible();
    expect(within(models).getAllByRole('listitem').length).toBeLessThan(80);
    act(() => useLibraryStore.setState({ layout: 'list' }));
    expect(within(models).getAllByRole('listitem').length).toBeLessThan(80);
  });

  it('lists every model, most used first, with the first on show', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    const models = await screen.findByRole('region', { name: 'Models' });
    expect(await within(models).findByText('8 models · most used first')).toBeVisible();
    const cards = within(models).getAllByRole('listitem');
    // Two projects each use Crystal lantern and Elsheria; a tie goes by name.
    expect(within(cards[1]!).getByRole('button')).toHaveAccessibleName('Elsheria');
    expect(within(cards[0]!).getByRole('button', { pressed: true })).toHaveAccessibleName(
      'Crystal lantern'
    );
    const side = screen.getByRole('complementary', { name: 'Selected model' });
    expect(within(side).getByRole('heading', { name: 'Crystal lantern' })).toBeVisible();
    expect(within(side).getByRole('button', { name: 'Update to v2' })).toBeVisible();
  });

  it('searches and filters, and says when nothing matches', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    const filters = await screen.findByRole('region', { name: 'Filters' });
    await userEvent.click(await within(filters).findByRole('button', { name: /^statues/ }));
    const models = screen.getByRole('region', { name: 'Models' });
    expect(within(models).getByText('2 of 8 models · most used first')).toBeVisible();
    await userEvent.type(
      screen.getByRole('searchbox', { name: 'Find a model' }),
      'nothing like it'
    );
    expect(within(models).getByText(/No model matches/)).toBeVisible();
  });

  it('teaches the first step when the library is empty', async () => {
    await openMockLibrary();
    render(<LibraryView />);
    expect(await screen.findByText('The library is empty')).toBeVisible();
    await userEvent.click(screen.getAllByRole('button', { name: 'Add from a project' })[1]!);
    expect(useLibraryStore.getState().adding).toBe('pick');
  });

  it('uses the selected model in the open project, spending nothing', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    const models = await screen.findByRole('region', { name: 'Models' });
    await userEvent.click(await within(models).findByRole('button', { name: 'Guardian statue' }));
    const side = screen.getByRole('complementary', { name: 'Selected model' });
    await userEvent.click(within(side).getByRole('button', { name: 'Add, no credits' }));
    expect(useAppStore.getState().notice?.title).toBe(
      'Added Guardian statue to Frostveil, no credits'
    );
  });

  it('switches to the list', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    await userEvent.click(await screen.findByRole('button', { name: 'List' }));
    expect(useLibraryStore.getState().layout).toBe('list');
    expect(await screen.findByText('12.9k triangles · 400 KB')).toBeVisible();
  });
});
