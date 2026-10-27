import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { registerMockFile } from '@/ipc/files';
import { runningJobs } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { QueueRow } from '.';

const [done, , , , running] = runningJobs('moon-island', 0);
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
});

describe('a queue row', () => {
  it('cancels a running job and shows how long it has run', async () => {
    const cancel = vi.fn(async () => {});
    useLabStore.setState({ cancel });
    const now = running!.startedAtMs! + 41_000;
    render(<QueueRow job={running!} projectDir="/w" now={now} />);
    expect(screen.getByText('41s')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Cancel Concept 05' }));
    expect(cancel).toHaveBeenCalledWith(running!.id);
  });

  it('opens a finished image in the lightbox', async () => {
    const openLightbox = vi.fn();
    useLabStore.setState({ openLightbox });
    const path = `/w/${done!.file}`;
    registerMockFile(path, '/thumb.png');
    render(<QueueRow job={done!} projectDir="/w" now={0} />);
    await userEvent.click(screen.getByRole('button', { name: 'Open Concept 01' }));
    expect(openLightbox).toHaveBeenCalledWith(path);
  });

  it('marks a failed job and runs it again with Retry', async () => {
    const retry = vi.fn(async () => {});
    useLabStore.setState({ retry });
    const failed = { ...done!, state: 'failed' as const, message: 'Codex stopped' };
    render(<QueueRow job={failed} projectDir="/nowhere" now={0} />);
    expect(screen.queryByRole('button', { name: /Cancel/ })).toBeNull();
    expect(screen.getByText('Codex stopped')).toHaveClass('text-danger');
    await userEvent.click(screen.getByRole('button', { name: 'Retry Concept 01' }));
    expect(retry).toHaveBeenCalledWith(failed.id);
  });

  it('says when a rate limited job tries again, and offers Cancel and Retry now', () => {
    const [, , , , , , limited, gaveUp] = runningJobs('moon-island', 0, 'rate');
    const at = new Date(2026, 9, 26, 9, 5).getTime();
    render(<QueueRow job={{ ...limited!, retryAtMs: at }} projectDir="/w" now={0} />);
    expect(screen.getByText('Waiting, rate limited, retries at 09:05')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Cancel Concept 07' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry Concept 07 now' })).toBeInTheDocument();
    cleanup();
    render(<QueueRow job={gaveUp!} projectDir="/w" now={0} />);
    expect(screen.getByText(/Rate limited by Codex, 4 times in a row/)).toHaveClass('text-danger');
    expect(screen.getByRole('button', { name: 'Retry Concept 08' })).toBeInTheDocument();
  });
});
