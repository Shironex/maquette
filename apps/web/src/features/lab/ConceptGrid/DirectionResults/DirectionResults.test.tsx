import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { DirectionResults } from '.';

const directions = labDirections(false);
const porch = directions.find(d => d.id === 'moon-island-porch')!;
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
  useLabStore.setState(lab, true);
});

describe('the results of a direction', () => {
  it('sums up the direction and shows a tile per image', () => {
    render(<DirectionResults view={view} direction={porch} />);
    expect(screen.getByText('6 concepts · 2 starred · 1 picked')).toBeVisible();
    expect(screen.getAllByRole('listitem')).toHaveLength(6);
  });

  it('asks what the asset is before sending the picks to Refs', async () => {
    const sendPicks = vi.fn(async () => 'moon-island-porch');
    useLabStore.setState({ sendPicks });
    render(<DirectionResults view={view} direction={porch} />);
    await userEvent.click(screen.getByRole('button', { name: 'Send picks to Refs' }));
    expect(screen.getByText('Adds Moon island porch to the Board at Refs')).toBeVisible();
    await userEvent.click(screen.getByRole('menuitem', { name: 'As a prop' }));
    expect(sendPicks).toHaveBeenCalledWith('prop');
  });

  it('holds the send buttons until a concept is picked', () => {
    render(<DirectionResults view={view} direction={train} />);
    expect(screen.getByRole('button', { name: 'Send picks to Refs' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Send to 3D' })).toHaveAttribute(
      'title',
      'Pick at least one concept first'
    );
  });
});
