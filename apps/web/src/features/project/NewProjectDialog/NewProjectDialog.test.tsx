import { act, cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProjectsStore } from '@/state/use-projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { NewProjectDialog } from '.';

afterEach(() => {
  cleanup();
  clearMocks();
  useProjectsStore.setState({ dialog: null, creating: false });
});

describe('the new project dialog', () => {
  it('renders nothing until the store opens it', async () => {
    await openMockWorkspace();
    render(<NewProjectDialog />);
    expect(screen.queryByRole('dialog')).toBeNull();
    act(() => useProjectsStore.getState().openDialog('template'));
    expect(await screen.findByRole('dialog', { name: 'New project' })).toBeVisible();
  });

  it('opens on Import a folder and closes from Cancel', async () => {
    await openMockWorkspace();
    useProjectsStore.setState({ dialog: 'import' });
    render(<NewProjectDialog />);
    expect(screen.getByRole('dialog', { name: 'Import a folder' })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(useProjectsStore.getState().dialog).toBeNull();
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
