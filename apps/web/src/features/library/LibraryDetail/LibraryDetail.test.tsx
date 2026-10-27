import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryDetail from './LibraryDetail';

const item = (id: string) => useLibraryStore.getState().view!.items.find(i => i.entry.id === id)!;

beforeEach(async () => {
  await openMockLibrary('demo');
  await useLibraryStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({ view: null, updating: null, plan: null });
});

describe('the selected library model', () => {
  it('lists its versions, the projects that use it and what reuse saved', () => {
    render(<LibraryDetail item={item('crystal-lantern')} />);
    expect(screen.getByText('Versions · 2')).toBeVisible();
    expect(screen.getByText('Frost cathedral')).toBeVisible();
    expect(screen.getByText('current')).toBeVisible();
    expect(screen.getByText('30 (1 reuse)')).toBeVisible();
  });

  it('opens the update plan for a copy on an older version', async () => {
    render(<LibraryDetail item={item('crystal-lantern')} />);
    await userEvent.click(screen.getByRole('button', { name: 'Update to v2' }));
    expect(useLibraryStore.getState().updating).toEqual({
      project: 'frost-cathedral',
      asset: 'crystal-lantern',
    });
  });

  it('keeps an .obj model out of a website project and says why', () => {
    render(<LibraryDetail item={item('crypt-brazier')} />);
    expect(screen.getByRole('button', { name: 'Add, no credits' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(
      screen.getByText(/builds its scene from .glb models, and this one is .obj/)
    ).toBeVisible();
  });
});
