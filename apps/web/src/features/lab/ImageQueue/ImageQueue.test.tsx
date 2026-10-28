import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { runningJobs } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { ImageQueue } from '.';
import { EMPTY_STYLE } from '@/state/lab-style';

const view: LabView = {
  project: 'frostveil',
  projectDir: '/w/frostveil',
  directions: [],
  jobs: runningJobs('frostveil', 0),
  refs: [],
  concurrency: 4,
  style: EMPTY_STYLE,
};
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
});

describe('the image queue', () => {
  it('says it is empty before the first job', () => {
    render(<ImageQueue view={{ ...view, jobs: [] }} />);
    expect(screen.getByText('Empty')).toBeVisible();
    expect(screen.getByText('Idle, 4 at a time')).toBeVisible();
  });

  it('counts the jobs and cancels the running ones', async () => {
    const cancelAll = vi.fn(async () => {});
    useLabStore.setState({ cancelAll });
    render(<ImageQueue view={view} />);
    expect(screen.getByText('4 of 6 done')).toBeVisible();
    expect(screen.getAllByRole('listitem')).toHaveLength(6);
    await userEvent.click(screen.getByRole('button', { name: 'Cancel running' }));
    expect(cancelAll).toHaveBeenCalledTimes(1);
  });

  it('clears the finished jobs once nothing is live', async () => {
    const clearFinished = vi.fn(async () => {});
    useLabStore.setState({ clearFinished });
    render(<ImageQueue view={{ ...view, jobs: view.jobs.slice(0, 4) }} />);
    await userEvent.click(screen.getByRole('button', { name: 'Clear' }));
    expect(clearFinished).toHaveBeenCalledTimes(1);
  });
});
