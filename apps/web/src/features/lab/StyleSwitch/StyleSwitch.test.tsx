import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { frostveilStyleBoard } from '@/ipc/mock/style-board';
import { EMPTY_STYLE } from '@/state/lab-style';
import { useLabStore } from '@/state/use-lab-store';
import { StyleSwitch } from '.';

const view: LabView = {
  project: 'frostveil',
  projectDir: '/w/frostveil',
  directions: [],
  jobs: [],
  refs: [],
  concurrency: 4,
  style: frostveilStyleBoard(),
};
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
});

describe('the style switch', () => {
  it('shows the board and turns the style off for this direction', async () => {
    const onChange = vi.fn();
    render(<StyleSwitch view={view} on onChange={onChange} />);
    const toggle = screen.getByRole('switch', { name: /^Use the project style/ });
    expect(toggle).toBeChecked();
    expect(screen.getByText('Revision 3, after your prompt')).toBeVisible();
    expect(screen.getByText('frosted glass, cool night light, chunky silhouettes')).toBeVisible();
    expect(screen.getByText('3 images · 4 colours')).toBeVisible();
    await userEvent.click(toggle);
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it('says the direction keeps its own look while off', () => {
    render(<StyleSwitch view={view} on={false} onChange={() => {}} />);
    expect(screen.getByRole('switch', { name: /^Use the project style/ })).not.toBeChecked();
    expect(screen.getByText('This direction keeps its own look')).toBeVisible();
  });

  it('opens the style board', async () => {
    const openStyle = vi.fn();
    useLabStore.setState({ openStyle });
    render(<StyleSwitch view={view} on onChange={() => {}} />);
    await userEvent.click(screen.getByRole('button', { name: 'Open the style board' }));
    expect(openStyle).toHaveBeenCalledTimes(1);
  });

  it('offers to set up an empty board, or stays out of the way', async () => {
    const openStyle = vi.fn();
    useLabStore.setState({ openStyle });
    const empty = { ...view, style: EMPTY_STYLE };
    const { container } = render(<StyleSwitch view={empty} on onChange={() => {}} />);
    expect(container).toBeEmptyDOMElement();
    cleanup();
    render(<StyleSwitch view={empty} on onChange={() => {}} offerSetup />);
    expect(screen.queryByRole('switch')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Set up' }));
    expect(openStyle).toHaveBeenCalledTimes(1);
  });
});
