import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ShellStatusBar } from '.';

afterEach(() => clearMocks());

describe('the shell status bar', () => {
  it('names the inbox folder outside the Lab and says when there are no runs', async () => {
    await openMockWorkspace();
    useAppStore.setState({ view: 'board' });
    render(<ShellStatusBar />);
    const workspace = useAppStore.getState().workspace!;
    expect(screen.getByText('Local worker connected')).toBeVisible();
    expect(
      screen.getByText(`Inbox at ${workspace.display}/${workspace.config.layout.inbox}`)
    ).toBeVisible();
    expect(screen.getByText('No runs yet')).toBeVisible();
  });

  it('leaves the inbox out in the Lab', async () => {
    await openMockWorkspace();
    useAppStore.setState({ view: 'lab' });
    render(<ShellStatusBar />);
    expect(screen.queryByText(/^Inbox at/)).toBeNull();
  });

  it('reads the last run out in a polite status region', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frostveil');
    render(<ShellStatusBar />);
    expect(screen.getByRole('status')).toHaveTextContent(/^Run \d+ failed at Capture posters$/);
  });
});
