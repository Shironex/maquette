import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useTrashStore } from '@/state/use-trash-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import RecentlyRemoved from './RecentlyRemoved';

beforeEach(async () => {
  await openMockWorkspace();
});

afterEach(() => {
  clearMocks();
  useTrashStore.setState({ project: null, entries: [], restoring: null });
  useAppStore.setState({ notice: null });
});

describe('RecentlyRemoved', () => {
  it('shows nothing while the trash is empty', async () => {
    const { container } = render(<RecentlyRemoved />);
    await vi.waitFor(() => expect(useTrashStore.getState().project).toBe('moon-island'));
    expect(container).toBeEmptyDOMElement();
  });

  it('lists a removed asset and restores it', async () => {
    await useTrashStore.getState().remove('moon-island', ['character']);
    render(<RecentlyRemoved />);
    const button = await screen.findByRole('button', { name: /Removed/ });
    expect(button).toHaveTextContent('1');
    await userEvent.click(button);
    const item = screen.getByRole('menuitem', { name: 'Character' });
    await userEvent.click(item);
    await vi.waitFor(() =>
      expect(
        (useAppStore.getState().workspace?.project?.assets ?? []).some(a => a.id === 'character')
      ).toBe(true)
    );
    await vi.waitFor(() => expect(screen.queryByRole('button', { name: /Removed/ })).toBeNull());
  });
});
