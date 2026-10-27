import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { ImageTile } from '.';

const directions = labDirections(false);
const arena = directions.find(d => d.id === 'frozen-lake-arena')!;
const first = arena.images.find(i => i.index === 1)!;
const view: LabView = {
  project: 'frostveil',
  projectDir: '/w/frostveil',
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
    render(<ImageTile view={view} direction={arena} image={first} />);
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
    render(<ImageTile view={view} direction={arena} image={first} />);
    const path = `/w/frostveil/${first.file}`;
    await userEvent.click(screen.getByRole('button', { name: 'Open Concept 01' }));
    expect(openLightbox).toHaveBeenCalledWith(path);
    await userEvent.click(screen.getByRole('button', { name: 'Make object refs from Concept 01' }));
    expect(openObjects).toHaveBeenCalledWith({
      project: 'frostveil',
      source: first.file,
      sourcePath: path,
      sourceName: 'Frozen lake arena',
    });
  });
});
