import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { DirectionRow } from '.';

const directions = labDirections(false);
const train = directions.find(d => d.id === 'late-train')!;
const view: LabView = {
  project: 'moon-island',
  projectDir: '/w/moon-island',
  directions,
  jobs: [],
  refs: [],
  concurrency: 4,
};
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  clearMocks();
  useLabStore.setState(lab, true);
});

function renderRow(menuOpen: boolean, onMenu = vi.fn(), at = view) {
  render(
    <ul>
      <DirectionRow view={at} direction={train} menuOpen={menuOpen} onMenu={onMenu} />
    </ul>
  );
  return onMenu;
}

describe('an earlier direction', () => {
  it('counts its images and marks the selected one', () => {
    useLabStore.setState({ selected: 'late-train' });
    renderRow(false);
    expect(screen.getByRole('button', { name: /^Late train/ })).toHaveAttribute(
      'aria-current',
      'true'
    );
    expect(screen.getByText('2 images')).toBeVisible();
  });

  it('opens its menu through onMenu', async () => {
    const onMenu = renderRow(false);
    await userEvent.click(screen.getByRole('button', { name: 'More for Late train' }));
    expect(onMenu).toHaveBeenCalledWith(true);
  });

  it('moves the direction to another project', async () => {
    await openMockWorkspace();
    const moveDirection = vi.fn(async () => true);
    useLabStore.setState({ moveDirection });
    renderRow(true);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Move to project…' }));
    expect(screen.getByText('Its concepts, picks, stars and Board card go along.')).toBeVisible();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rooftop cinema' }));
    expect(moveDirection).toHaveBeenCalledWith('late-train', 'rooftop-cinema');
  });

  it('holds the move while its images are generating', async () => {
    const job = { direction: 'late-train', state: 'running' } as LabView['jobs'][number];
    renderRow(true, vi.fn(), { ...view, jobs: [job] });
    await userEvent.click(screen.getByRole('menuitem', { name: 'Move to project…' }));
    expect(screen.getByText(/still generating/)).toBeVisible();
  });
});
