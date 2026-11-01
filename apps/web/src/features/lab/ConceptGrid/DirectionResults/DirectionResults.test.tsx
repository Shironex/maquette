import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { DirectionResults } from '.';
import { tooltipText } from '@/test/tooltipText';
import { EMPTY_STYLE } from '@/state/lab-style';
import { frostveilStyleBoard } from '@/ipc/mock/style-board';

const directions = labDirections(false);
const arena = directions.find(d => d.id === 'frozen-lake-arena')!;
const train = directions.find(d => d.id === 'veil-gate-kit')!;
const view: LabView = {
  project: 'frostveil',
  projectDir: '/w/frostveil',
  directions,
  jobs: [],
  refs: [],
  concurrency: 4,
  style: EMPTY_STYLE,
};
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
});

describe('the results of a direction', () => {
  it('sums up the direction and shows a tile per image', () => {
    render(<DirectionResults view={view} direction={arena} />);
    expect(screen.getByText('6 concepts · 2 starred · 1 picked')).toBeVisible();
    expect(screen.getAllByRole('listitem')).toHaveLength(6);
  });

  it('asks what the asset is before sending the picks to Refs', async () => {
    const sendPicks = vi.fn(async () => 'frozen-lake-arena');
    useLabStore.setState({ sendPicks });
    render(<DirectionResults view={view} direction={arena} />);
    await userEvent.click(screen.getByRole('button', { name: 'Send picks to Refs' }));
    expect(screen.getByText('Adds Frozen lake arena to the Board at Refs')).toBeVisible();
    await userEvent.click(screen.getByRole('menuitem', { name: 'As a prop' }));
    expect(sendPicks).toHaveBeenCalledWith('prop');
  });

  it('offers cutting a scene into objects before sending it whole', async () => {
    const openObjects = vi.fn();
    useLabStore.setState({ openObjects });
    render(<DirectionResults view={view} direction={arena} />);
    await userEvent.click(screen.getByRole('button', { name: 'Send picks to Refs' }));
    const rows = screen.getAllByRole('menuitem').map(r => r.textContent);
    expect(rows[0]).toContain('Cut into objects first');
    expect(screen.getByRole('menuitem', { name: 'As a scene' })).toHaveAccessibleDescription(
      'Everything in it fuses into one mesh'
    );
    await userEvent.click(screen.getByRole('menuitem', { name: 'Cut into objects first' }));
    expect(openObjects).toHaveBeenCalledWith(
      expect.objectContaining({ project: view.project, sourceName: 'Frozen lake arena' })
    );
  });

  it('holds the send buttons until a concept is picked', async () => {
    render(<DirectionResults view={view} direction={train} />);
    expect(screen.getByRole('button', { name: 'Send picks to Refs' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(await tooltipText(screen.getByRole('button', { name: 'Send to 3D' }))).toBe(
      'Pick at least one concept first'
    );
  });

  it('chips the style board revision the direction carried', () => {
    const style = frostveilStyleBoard();
    render(
      <DirectionResults view={{ ...view, style }} direction={{ ...arena, styleRevision: 2 }} />
    );
    const chip = screen.getByText('Style r2');
    expect(chip).toHaveAttribute(
      'title',
      'Made with the project style at revision 2; the board is at 3 now'
    );
  });
});
