import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { useWorkspaceLock } from '@/state/use-workspace-lock';
import { WorkspaceInUseDialog } from '.';

const message =
  'This workspace is open on Studio PC. Close Maquette there first, or Syncthing may create conflict copies.';

describe('the workspace in use dialog', () => {
  beforeEach(() => useWorkspaceLock.setState({ question: null }));

  it('shows nothing until asked', () => {
    render(<WorkspaceInUseDialog />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('names the other computer and answers Open anyway or Cancel', async () => {
    render(<WorkspaceInUseDialog />);
    let answer: Promise<boolean> = Promise.resolve(false);
    act(() => {
      answer = useWorkspaceLock.getState().ask({ root: '/ws', message });
    });
    expect(screen.getByRole('dialog', { name: 'Open this workspace here too?' })).toHaveTextContent(
      message
    );
    expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
    await userEvent.click(screen.getByRole('button', { name: 'Open anyway' }));
    await expect(answer).resolves.toBe(true);
    expect(screen.queryByRole('dialog')).toBeNull();

    act(() => {
      answer = useWorkspaceLock.getState().ask({ root: '/ws', message });
    });
    await userEvent.keyboard('{Escape}');
    await expect(answer).resolves.toBe(false);
  });
});
