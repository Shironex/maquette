import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections, labRefs } from '@/ipc/mock/lab-fixtures';
import { frostveilStyleBoard } from '@/ipc/mock/style-board';
import { StyleImages } from '.';

const board = frostveilStyleBoard();
const view: LabView = {
  project: 'frostveil',
  projectDir: '/w/frostveil',
  directions: labDirections(false),
  jobs: [],
  refs: labRefs(),
  concurrency: 4,
  style: board,
};
const files = board.images.map(i => i.file);

afterEach(cleanup);

describe('the style images', () => {
  it('names each image after its source and removes one', async () => {
    const onRemove = vi.fn();
    render(<StyleImages view={view} images={files} onAdd={() => {}} onRemove={onRemove} />);
    expect(screen.getByText('3 of 4')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Remove frozen-lake-concept.png' }));
    expect(onRemove).toHaveBeenCalledWith('style/frozen_lake_concept.png');
  });

  it('offers project images that are not on the board yet', async () => {
    const onAdd = vi.fn();
    render(<StyleImages view={view} images={files} onAdd={onAdd} onRemove={() => {}} />);
    await userEvent.click(screen.getByRole('button', { name: 'Add style image' }));
    // The ref already on the board is not offered again.
    expect(screen.queryByRole('button', { name: 'Add frozen-lake-concept.png' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Add elsheria-model.png' }));
    expect(onAdd).toHaveBeenCalledWith('refs/elsheria-model.png');
  });

  it('has no Add once the board holds four', () => {
    const four = [...files, 'refs/elsheria-model.png'];
    render(<StyleImages view={view} images={four} onAdd={() => {}} onRemove={() => {}} />);
    expect(screen.queryByRole('button', { name: 'Add style image' })).toBeNull();
  });
});
