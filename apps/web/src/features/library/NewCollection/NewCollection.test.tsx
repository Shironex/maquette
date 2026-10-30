import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import NewCollection from './NewCollection';

afterEach(() => clearMocks());

describe('a new collection', () => {
  it('makes a collection with the picked models from its name field', async () => {
    await openMockLibrary('demo');
    render(<NewCollection entries={['veil-platform']} />);
    await userEvent.click(screen.getByRole('button', { name: /New collection/ }));
    await userEvent.type(
      screen.getByRole('textbox', { name: 'New collection name' }),
      'Night set{Enter}'
    );
    await waitFor(() =>
      expect(screen.getByRole('button', { name: /New collection/ })).toBeVisible()
    );
    const made = (await commands.libraryGet()).collections.find(c => c.name === 'Night set');
    expect(made?.entries).toEqual(['veil-platform']);
  });

  it('lets the name go with Escape', async () => {
    await openMockLibrary('demo');
    render(<NewCollection entries={[]} />);
    await userEvent.click(screen.getByRole('button', { name: /New collection/ }));
    await userEvent.type(screen.getByRole('textbox', { name: 'New collection name' }), 'x{Escape}');
    expect(screen.queryByRole('textbox')).toBeNull();
    expect((await commands.libraryGet()).collections).toEqual([]);
  });
});
