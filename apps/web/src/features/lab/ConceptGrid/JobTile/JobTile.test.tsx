import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { labDirections, runningJobs } from '@/ipc/mock/lab-fixtures';
import { JobTile } from '.';

const porch = labDirections(true).find(d => d.id === 'moon-island-porch')!;
const [, , , , running] = runningJobs('moon-island', 0);

afterEach(cleanup);

describe('a job tile', () => {
  it('shows a generating job with its progress', () => {
    render(<JobTile direction={porch} job={running!} />);
    expect(screen.getByText('generating')).toBeVisible();
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(screen.getByText('Concept 05')).toBeVisible();
  });

  it('shows a queued job as in line', () => {
    render(<JobTile direction={porch} job={{ ...running!, state: 'queued', startedAtMs: null }} />);
    expect(screen.getByText('waiting')).toBeVisible();
    expect(screen.getByText('in line')).toBeVisible();
    expect(screen.queryByRole('progressbar')).toBeNull();
  });
});
