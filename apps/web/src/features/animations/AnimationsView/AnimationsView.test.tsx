import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { NO_ANIMATION_FILTERS } from '@/state/animations';
import { useAnimationsStore } from '@/state/use-animations-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import AnimationsView from './AnimationsView';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

afterEach(() => {
  clearMocks();
  useAnimationsStore.setState({
    view: null,
    filters: NO_ANIMATION_FILTERS,
    selected: null,
    importing: null,
    layout: 'grid',
  });
});

describe('the Animations view', () => {
  it('lists every clip, newest first, with the first on show', async () => {
    await openMockLibrary('demo');
    render(<AnimationsView />);
    const clips = await screen.findByRole('region', { name: 'Clips' });
    expect(await within(clips).findByText('16 clips · newest first')).toBeVisible();
    expect(within(clips).getByText('4 packs, 4 singles')).toBeVisible();
    const first = within(clips).getAllByRole('listitem')[0]!;
    expect(within(first).getByRole('button', { pressed: true })).toHaveAccessibleName(
      'Standing Dodge Left'
    );
    const side = screen.getByRole('complementary', { name: 'Selected clip' });
    expect(within(side).getByRole('heading', { name: 'Standing Dodge Left' })).toBeVisible();
  });

  it('renders only the clips in view of a long library, in the grid and the list', async () => {
    await openMockLibrary('demo');
    render(<AnimationsView />);
    const clips = await screen.findByRole('region', { name: 'Clips' });
    await within(clips).findByText('16 clips · newest first');
    const view = useAnimationsStore.getState().view!;
    const one = view.items[0]!;
    const items = Array.from({ length: 500 }, (_, i) => ({
      ...one,
      clip: { ...one.clip, id: `copy-${i}`, name: `Copy ${i}` },
    }));
    act(() => useAnimationsStore.setState({ view: { ...view, items } }));
    expect(within(clips).getByText('500 clips · newest first')).toBeVisible();
    const inGrid = within(clips).getAllByRole('listitem').length;
    expect(inGrid).toBeGreaterThan(0);
    expect(inGrid).toBeLessThan(80);
    act(() => useAnimationsStore.setState({ layout: 'list' }));
    const inList = within(clips).getAllByRole('listitem').length;
    expect(inList).toBeLessThan(80);
    expect(within(clips).getByRole('button', { name: /^Copy 0\b/ })).toBeVisible();
  });

  it('filters by pack and loops, and says when nothing matches', async () => {
    await openMockLibrary('demo');
    render(<AnimationsView />);
    const filters = await screen.findByRole('region', { name: 'Filters' });
    await userEvent.click(await within(filters).findByRole('button', { name: /^Pro Magic Pack/ }));
    const clips = screen.getByRole('region', { name: 'Clips' });
    expect(within(clips).getByText('4 of 16 clips · newest first')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Loops' }));
    expect(within(clips).getByText('1 of 16 clips · newest first')).toBeVisible();
    await userEvent.type(screen.getByRole('searchbox', { name: 'Find a clip' }), 'nothing');
    expect(within(clips).getByText(/No clip matches/)).toBeVisible();
  });

  it('switches to the list and opens the import sheet', async () => {
    await openMockLibrary('demo');
    render(<AnimationsView />);
    await screen.findByRole('region', { name: 'Clips' });
    await userEvent.click(screen.getByRole('button', { name: 'List' }));
    expect(screen.getByRole('button', { name: 'Run To Rolling' })).toHaveTextContent('Singles');
    await userEvent.click(screen.getByRole('button', { name: 'Import clips' }));
    expect(useAnimationsStore.getState().importing).not.toBeNull();
  });

  it('teaches the ways in when the library is empty', async () => {
    await openMockLibrary();
    render(<AnimationsView />);
    expect(await screen.findByText('No clips yet')).toBeVisible();
    expect(screen.getAllByRole('button', { name: 'Import clips' })).toHaveLength(2);
  });
});
