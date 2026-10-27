import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { EarlierDirections } from '.';

const view: LabView = {
  project: 'frostveil',
  projectDir: '/w/frostveil',
  directions: labDirections(false),
  jobs: [],
  refs: [],
  concurrency: 4,
};
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
});

describe('the earlier directions', () => {
  it('lists every direction and reuses the one clicked', async () => {
    const reuse = vi.fn();
    useLabStore.setState({ reuse });
    render(<EarlierDirections view={view} />);
    expect(screen.getAllByRole('listitem')).toHaveLength(view.directions.length);
    await userEvent.click(screen.getByRole('button', { name: /^Veil gate kit/ }));
    expect(reuse).toHaveBeenCalledWith('veil-gate-kit');
  });

  it('keeps one menu open at a time', async () => {
    render(<EarlierDirections view={view} />);
    await userEvent.click(screen.getByRole('button', { name: 'More for Veil gate kit' }));
    expect(screen.getByRole('button', { name: 'More for Veil gate kit' })).toHaveAttribute(
      'aria-expanded',
      'true'
    );
    await userEvent.click(screen.getByRole('button', { name: 'More for Elsheria sheets' }));
    expect(screen.getByRole('button', { name: 'More for Veil gate kit' })).toHaveAttribute(
      'aria-expanded',
      'false'
    );
  });
});
