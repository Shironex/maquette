import { act, cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { labDirections } from '@/ipc/mock/lab-fixtures';
import { useLabStore } from '@/state/use-lab-store';
import { Lightbox } from '.';
import { EMPTY_STYLE } from '@/state/lab-style';

const directions = labDirections(false);
const view: LabView = {
  project: 'frostveil',
  projectDir: '/w',
  directions,
  jobs: [],
  refs: [],
  concurrency: 4,
  style: EMPTY_STYLE,
};
const arena = directions.find(d => d.id === 'frozen-lake-arena')!;
const file = (index: number) => arena.images.find(i => i.index === index)!.file;
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
});

function openOn(index: number) {
  useLabStore.setState({ selected: 'frozen-lake-arena', lightbox: `/w/${file(index)}` });
}

describe('the lightbox', () => {
  it('renders nothing while closed', () => {
    const { container } = render(<Lightbox view={view} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('steps through the direction with the arrows and closes on Escape', async () => {
    openOn(1);
    render(<Lightbox view={view} />);
    expect(screen.getByRole('dialog', { name: 'Frozen lake arena · Concept 01' })).toBeVisible();
    expect(screen.getByText('1 of 6')).toBeVisible();
    await userEvent.keyboard('{ArrowRight}');
    expect(screen.getByText('2 of 6')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Previous image' }));
    await userEvent.click(screen.getByRole('button', { name: 'Previous image' }));
    expect(screen.getByText('6 of 6')).toBeVisible();
    await userEvent.keyboard('{Escape}');
    expect(useLabStore.getState().lightbox).toBeNull();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('stars and picks with S and P', async () => {
    const mark = vi.fn(async () => {});
    useLabStore.setState({ mark });
    act(() => openOn(2));
    render(<Lightbox view={view} />);
    await userEvent.keyboard('s');
    expect(mark).toHaveBeenCalledWith(file(2), { starred: false });
    await userEvent.keyboard('p');
    expect(mark).toHaveBeenCalledWith(file(2), { picked: true });
  });

  it('shows a contact sheet on its own, by file name', () => {
    useLabStore.setState({ lightbox: '/w/concepts/sheet.png' });
    render(<Lightbox view={view} />);
    expect(screen.getByRole('dialog', { name: 'sheet.png' })).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Next image' })).toBeNull();
  });
});
