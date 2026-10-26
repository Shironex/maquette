import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LibraryView } from '@/ipc/client';
import { NO_FILTERS, NOT_USED } from '@/state/library';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { commands } from '@/ipc/client';
import LibraryFilters from './LibraryFilters';

afterEach(() => clearMocks());

async function items(): Promise<LibraryView['items']> {
  await openMockLibrary('demo');
  return (await commands.libraryGet()).items;
}

describe('the library filters', () => {
  it('counts each tag, source and project, and toggles a pick off again', async () => {
    const onChange = vi.fn();
    const all = await items();
    const { rerender } = render(
      <LibraryFilters items={all} filters={NO_FILTERS} onChange={onChange} />
    );
    expect(screen.getByRole('button', { name: /^props ?3$/ })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: /^Built ?1$/ }));
    expect(onChange).toHaveBeenLastCalledWith({ source: 'built' });
    rerender(
      <LibraryFilters
        items={all}
        filters={{ ...NO_FILTERS, source: 'built' }}
        onChange={onChange}
      />
    );
    await userEvent.click(screen.getByRole('button', { name: /^Built ?1$/ }));
    expect(onChange).toHaveBeenLastCalledWith({ source: null });
    await userEvent.click(screen.getByRole('button', { name: /^Not used/ }));
    expect(onChange).toHaveBeenLastCalledWith({ usedIn: NOT_USED });
  });
});
