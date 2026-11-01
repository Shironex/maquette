import { act, cleanup, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections } from '@/ipc/mock/lab-fixtures';
import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
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
  clearMocks();
  useAppStore.setState({ workspace: null });
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

  it('still asks the kind when the card only exists because images were dropped', async () => {
    await openMockWorkspace();
    const project = useAppStore.getState().workspace!.project!;
    const thumbnail = 'concepts/frozen-lake-arena/frozen-lake-arena_01.png';
    act(() =>
      useAppStore.getState().setProject({
        ...project,
        assets: [{ ...project.assets![0]!, name: arena.name, stage: 'concept', thumbnail }],
      })
    );
    const sendPicks = vi.fn(async () => 'frozen-lake-arena');
    useLabStore.setState({ sendPicks });
    render(<DirectionResults view={view} direction={arena} />);
    await userEvent.click(screen.getByRole('button', { name: 'Send picks to Refs' }));
    expect(screen.getByRole('menuitem', { name: 'As a prop' })).toBeVisible();
    expect(sendPicks).not.toHaveBeenCalled();
  });

  it('keeps the cut out from under the line about adding to the Board', async () => {
    render(<DirectionResults view={view} direction={arena} />);
    await userEvent.click(screen.getByRole('button', { name: 'Send picks to Refs' }));
    const adds = screen.getByRole('group', { name: 'Adds Frozen lake arena to the Board at Refs' });
    expect(within(adds).getByRole('menuitem', { name: 'As a prop' })).toBeVisible();
    expect(within(adds).queryByRole('menuitem', { name: /Cut/ })).toBeNull();
    expect(
      screen.getByRole('menuitem', { name: 'Cut into objects first' })
    ).toHaveAccessibleDescription(/adds nothing to the Board yet/);
  });

  it('says the cut works on the first of several picks', async () => {
    const two = {
      ...arena,
      images: arena.images.map((image, i) => ({ ...image, picked: i < 2 })),
    };
    const openObjects = vi.fn();
    useLabStore.setState({ openObjects });
    render(<DirectionResults view={view} direction={two} />);
    await userEvent.click(screen.getByRole('button', { name: 'Send picks to Refs' }));
    const cut = screen.getByRole('menuitem', { name: 'Cut the first pick into objects' });
    expect(cut).toHaveAccessibleDescription(/first of your 2 picks/);
    await userEvent.click(cut);
    expect(openObjects).toHaveBeenCalledWith(
      expect.objectContaining({ source: two.images[0]!.file })
    );
  });

  it('points at object refs when a known card skips the menu', async () => {
    await openMockWorkspace();
    const project = useAppStore.getState().workspace!.project!;
    act(() =>
      useAppStore.getState().setProject({
        ...project,
        assets: [{ ...project.assets![0]!, name: arena.name, stage: 'refs', thumbnail: null }],
      })
    );
    render(<DirectionResults view={view} direction={arena} />);
    const send = screen.getByRole('button', { name: 'Send picks to Refs' });
    expect(await tooltipText(send)).toMatch(/Make object refs/);
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
