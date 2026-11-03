import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useProjectTrashStore } from '@/state/use-project-trash-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import RemovedProjectsDialog from './RemovedProjectsDialog';

beforeEach(async () => {
  await openMockWorkspace();
});

afterEach(() => {
  clearMocks();
  useProjectTrashStore.setState({ removed: [], asking: null, listOpen: false, busy: false });
  useProjectsStore.setState({ list: [], menuOpen: false, dialog: null });
  useAppStore.setState({ notice: null });
});

describe('RemovedProjectsDialog', () => {
  it('says when nothing was removed', async () => {
    render(<RemovedProjectsDialog />);
    useProjectTrashStore.getState().openList();
    expect(await screen.findByRole('dialog', { name: 'Removed projects' })).toHaveTextContent(
      'No removed projects'
    );
  });

  it('lists a removed project and restores it, opening it', async () => {
    expect(await useProjectTrashStore.getState().remove('frost-cathedral')).toBeNull();
    render(<RemovedProjectsDialog />);
    useProjectTrashStore.getState().openList();
    const list = await screen.findByRole('list', { name: 'Removed projects' });
    const row = within(list).getByRole('listitem');
    expect(row).toHaveTextContent(/Website · \d+ assets? · removed just now/);
    await userEvent.click(within(row).getByRole('button', { name: /^Restore Frost/ }));
    await vi.waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(useAppStore.getState().workspace?.project?.id).toBe('frost-cathedral');
    expect(useAppStore.getState().notice?.title).toMatch(/is back$/);
    expect(useProjectTrashStore.getState().removed).toEqual([]);
  });
});
