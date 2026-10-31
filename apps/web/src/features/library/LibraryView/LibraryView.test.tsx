import { act, render, screen, within, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { NO_FILTERS } from '@/state/library';
import { useAppStore } from '@/state/use-app-store';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { QuickView } from '@/features/viewer/QuickView';
import { useQuickViewStore } from '@/state/use-quick-view-store';
import LibraryView from './LibraryView';

vi.mock('@/features/viewer/ModelPreview', () => ({ default: () => null }));

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({
    view: null,
    filters: NO_FILTERS,
    selected: null,
    adding: null,
    layout: 'grid',
    group: 'none',
  });
  useQuickViewStore.setState({ target: null });
});

describe('the Library view', () => {
  it('says it is busy until the models load', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    expect(screen.getByRole('main')).toHaveAttribute('aria-busy', 'true');
    await waitFor(() => expect(screen.getByRole('main')).toHaveAttribute('aria-busy', 'false'));
  });

  it('renders only the models in view of a long library, in the grid and the list', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    const models = await screen.findByRole('region', { name: 'Models' });
    await within(models).findByText('8 models');
    const view = useLibraryStore.getState().view!;
    const one = view.items[0]!;
    const items = Array.from({ length: 300 }, (_, i) => ({
      ...one,
      entry: { ...one.entry, id: `copy-${i}`, name: `Copy ${i}` },
    }));
    act(() => useLibraryStore.setState({ view: { ...view, items } }));
    expect(within(models).getByText('300 models')).toBeVisible();
    expect(within(models).getAllByRole('listitem').length).toBeLessThan(80);
    act(() => useLibraryStore.setState({ layout: 'list' }));
    expect(within(models).getAllByRole('listitem').length).toBeLessThan(80);
  });

  it('lists every model, most used first, with the first on show', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    const models = await screen.findByRole('region', { name: 'Models' });
    expect(await within(models).findByText('8 models')).toBeVisible();
    const cards = within(models).getAllByRole('listitem');
    // Two projects each use Crystal lantern and Elsheria; a tie goes by name.
    expect(within(cards[1]!).getByRole('button', { pressed: false })).toHaveAccessibleName(
      'Elsheria'
    );
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
    expect(within(models).getByText('2 of 8 models')).toBeVisible();
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

  it('opens a model in the 3D quick view from its picture, and gives focus back on close', async () => {
    await openMockLibrary('demo');
    render(
      <>
        <LibraryView />
        <QuickView />
      </>
    );
    const picture = await screen.findByRole('button', { name: 'Inspect 3D model of Elsheria' });
    await userEvent.click(picture);
    const dialog = screen.getByRole('dialog', { name: 'Elsheria' });
    expect(dialog).toHaveAccessibleDescription('v3 · 34.7k triangles · 1.1 MB · Library');
    expect(useLibraryStore.getState().selected).toBe('elsheria');
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(picture).toHaveFocus();
    // The selected model's big picture opens it too.
    const side = screen.getByRole('complementary', { name: 'Selected model' });
    await userEvent.click(
      within(side).getByRole('button', { name: 'Inspect 3D model of Elsheria' })
    );
    expect(screen.getByRole('dialog', { name: 'Elsheria' })).toBeInTheDocument();
  });

  it('keeps an .obj model, which has no 3D preview, to selecting', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    await screen.findByRole('button', { name: 'Crypt brazier' });
    expect(screen.queryByRole('button', { name: 'Inspect 3D model of Crypt brazier' })).toBeNull();
  });

  it('puts the models under a header each when grouped by what made them', async () => {
    await openMockLibrary('demo');
    render(<LibraryView />);
    const models = await screen.findByRole('region', { name: 'Models' });
    await within(models).findByText('8 models');
    expect(within(models).queryByRole('heading', { level: 3 })).toBeNull();
    act(() => useLibraryStore.setState({ group: 'source' }));
    const heads = within(models).getAllByRole('heading', { level: 3 });
    expect(heads.length).toBeGreaterThan(0);
    const total = within(models)
      .getAllByText(/^\d+ models?$/)
      .filter(el => el.closest('h3'))
      .map(el => Number(el.textContent!.split(' ')[0]))
      .reduce((a, b) => a + b, 0);
    expect(total).toBe(8);
  });
});
