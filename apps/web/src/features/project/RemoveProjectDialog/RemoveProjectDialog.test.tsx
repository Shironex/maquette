import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useProjectTrashStore } from '@/state/use-project-trash-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import RemoveProjectDialog from './RemoveProjectDialog';

beforeEach(async () => {
  await openMockWorkspace();
});

afterEach(() => {
  clearMocks();
  useProjectTrashStore.setState({ removed: [], asking: null, listOpen: false, busy: false });
  useProjectsStore.setState({ list: [], menuOpen: false, dialog: null });
  useAppStore.setState({ notice: null });
});

describe('RemoveProjectDialog', () => {
  it('names the project, says the repo stays, and opens the next project after a remove', async () => {
    await useProjectsStore.getState().switchTo('frost-cathedral');
    render(<RemoveProjectDialog />);
    useProjectTrashStore.getState().ask('frost-cathedral');
    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveAccessibleName(/^Remove Frost/);
    expect(dialog).toHaveTextContent('moves to the workspace trash');
    expect(dialog).toHaveTextContent('Removed projects in the project switcher puts it back');
    expect(dialog).toHaveTextContent('is never touched');
    expect(dialog).toHaveTextContent('Its sends keep their records');
    expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();

    await userEvent.click(screen.getByRole('button', { name: 'Remove project' }));
    await vi.waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    const workspace = useAppStore.getState().workspace;
    expect(workspace?.projects.some(p => p.id === 'frost-cathedral')).toBe(false);
    expect(workspace?.project?.id).not.toBe('frost-cathedral');
    expect(useAppStore.getState().notice?.title).toMatch(/is in the workspace trash$/);
    expect(useAppStore.getState().notice?.action?.label).toBe('Undo');
    expect(useProjectTrashStore.getState().removed.map(r => r.id)).toEqual(['frost-cathedral']);
  });

  it('greys out Remove project with why while work runs in the project', async () => {
    render(<RemoveProjectDialog />);
    useProjectTrashStore.getState().ask('frostveil');
    await screen.findByRole('dialog');
    await vi.waitFor(() =>
      expect(screen.getByRole('note')).toHaveTextContent('Work is running in this project')
    );
    const remove = screen.getByRole('button', { name: 'Remove project' });
    expect(remove).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(remove);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(useAppStore.getState().workspace?.projects.some(p => p.id === 'frostveil')).toBe(true);
  });

  it('says why when the core refuses the remove, and greys the button out', async () => {
    // The switcher's rows never say that work runs in it, so only the core can.
    const load = useProjectsStore.getState().load;
    useProjectsStore.setState({ load: async () => {} });
    try {
      render(<RemoveProjectDialog />);
      useProjectTrashStore.getState().ask('frostveil');
      await userEvent.click(await screen.findByRole('button', { name: 'Remove project' }));
      expect(await screen.findByRole('alert')).toHaveTextContent(/not over in Frostveil/);
      const remove = screen.getByRole('button', { name: 'Remove project' });
      expect(remove).toHaveAttribute('aria-disabled', 'true');
      expect(useAppStore.getState().workspace?.projects.some(p => p.id === 'frostveil')).toBe(true);
    } finally {
      useProjectsStore.setState({ load });
    }
  });

  it('changes nothing on Cancel', async () => {
    render(<RemoveProjectDialog />);
    useProjectTrashStore.getState().ask('frost-cathedral');
    await userEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(useAppStore.getState().workspace?.projects).toHaveLength(3);
  });
});
