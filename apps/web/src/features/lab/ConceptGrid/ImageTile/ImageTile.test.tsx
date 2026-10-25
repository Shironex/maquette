import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { ImageTile } from '.';

const directions = labDirections(false);
const porch = directions.find(d => d.id === 'moon-island-porch')!;
const first = porch.images.find(i => i.index === 1)!;
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

describe('an image tile', () => {
  it('unpicks and unstars a picked, starred concept', async () => {
    const mark = vi.fn(async () => {});
    useLabStore.setState({ mark });
    render(<ImageTile view={view} direction={porch} image={first} />);
    expect(screen.getByText('Picked')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Unpick' }));
    expect(mark).toHaveBeenCalledWith(first.file, { picked: false });
    await userEvent.click(screen.getByRole('button', { name: 'Unstar Concept 01' }));
    expect(mark).toHaveBeenCalledWith(first.file, { starred: false });
  });

  it('opens the lightbox and Make object refs on the concept', async () => {
    const openLightbox = vi.fn();
    const openObjects = vi.fn();
    useLabStore.setState({ openLightbox, openObjects });
    render(<ImageTile view={view} direction={porch} image={first} />);
    const path = `/w/moon-island/${first.file}`;
    await userEvent.click(screen.getByRole('button', { name: 'Open Concept 01' }));
    expect(openLightbox).toHaveBeenCalledWith(path);
    await userEvent.click(screen.getByRole('button', { name: 'Make object refs from Concept 01' }));
    expect(openObjects).toHaveBeenCalledWith({
      project: 'moon-island',
      source: first.file,
      sourcePath: path,
      sourceName: 'Moon island porch',
    });
  });
});
