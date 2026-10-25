import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Button, IconButton, Kbd, Tooltip, tooltipPlacement } from '../index';
import { SKIP_DELAY } from '../components/feedback/Tooltip.hooks';

const wait = (ms: number) => new Promise(done => setTimeout(done, ms));

describe('tooltipPlacement', () => {
  const viewport = { width: 800, height: 600 };
  const tip = { width: 120, height: 26 };

  it('centres over the trigger on the side asked for', () => {
    const at = tooltipPlacement(
      { top: 300, left: 400, width: 40, height: 30 },
      tip,
      'top',
      viewport
    );
    expect(at).toEqual({ side: 'top', top: 300 - 8 - 26, left: 420 - 60 });
  });

  it('flips to the other side when the window edge is too close', () => {
    const at = tooltipPlacement({ top: 4, left: 400, width: 40, height: 30 }, tip, 'top', viewport);
    expect(at.side).toBe('bottom');
    expect(at.top).toBe(4 + 30 + 8);
  });

  it('is pushed back inside the window along the edge', () => {
    const left = tooltipPlacement(
      { top: 300, left: 2, width: 20, height: 20 },
      tip,
      'top',
      viewport
    );
    expect(left.left).toBe(8);
    const right = tooltipPlacement(
      { top: 300, left: 790, width: 10, height: 20 },
      tip,
      'bottom',
      viewport
    );
    expect(right.left).toBe(800 - 120 - 8);
  });

  it('flips a side tooltip from the right edge to the left', () => {
    const at = tooltipPlacement(
      { top: 300, left: 740, width: 40, height: 30 },
      tip,
      'right',
      viewport
    );
    expect(at.side).toBe('left');
    expect(at.left).toBe(740 - 8 - 120);
  });
});

describe('Tooltip', () => {
  // Let the delay group cool down between tests.
  beforeEach(() => wait(SKIP_DELAY + 50));

  it('shows after a short rest on hover and describes its trigger', async () => {
    render(
      <Tooltip label="Frame every model" shortcut="F">
        <button type="button">Frame</button>
      </Tooltip>
    );
    const button = screen.getByRole('button', { name: 'Frame' });
    await userEvent.hover(button);
    expect(screen.queryByRole('tooltip')).toBeNull();
    const tip = await screen.findByRole('tooltip');
    expect(tip).toHaveTextContent('Frame every model');
    expect(tip.querySelector('kbd')).toHaveTextContent('F');
    expect(button).toHaveAccessibleDescription(/Frame every model/);
    await userEvent.unhover(button);
    expect(screen.queryByRole('tooltip')).toBeNull();
    expect(button).not.toHaveAttribute('aria-describedby');
  });

  it('shows at once on keyboard focus and hides on Escape and blur', async () => {
    render(
      <>
        <IconButton icon="magnet" label="Snapping" />
        <button type="button">Next</button>
      </>
    );
    await userEvent.tab();
    expect(screen.getByRole('tooltip')).toHaveTextContent('Snapping');
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('tooltip')).toBeNull();
    await userEvent.tab();
    expect(screen.queryByRole('tooltip')).toBeNull();
    await userEvent.tab({ shift: true });
    expect(screen.getByRole('tooltip')).toHaveTextContent('Snapping');
  });

  it('hides on a press and stays hidden until the pointer leaves', async () => {
    const onClick = vi.fn();
    render(<IconButton icon="grid-four" label="Show the grid" onClick={onClick} />);
    const button = screen.getByRole('button', { name: 'Show the grid' });
    await userEvent.hover(button);
    await screen.findByRole('tooltip');
    await userEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
    await wait(600);
    expect(screen.queryByRole('tooltip')).toBeNull();
  });

  it('shows a neighbour at once inside the delay group, one at a time', async () => {
    render(
      <>
        <IconButton icon="arrow-u-up-left" label="Undo" />
        <IconButton icon="arrow-u-up-right" label="Redo" />
      </>
    );
    const undo = screen.getByRole('button', { name: 'Undo' });
    const redo = screen.getByRole('button', { name: 'Redo' });
    await userEvent.hover(undo);
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Undo');
    await userEvent.unhover(undo);
    await userEvent.hover(redo);
    expect(screen.getAllByRole('tooltip')).toHaveLength(1);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Redo');
  });

  it('renders the trigger alone when disabled', async () => {
    render(
      <Tooltip label="Hidden" disabled>
        <button type="button">Plain</button>
      </Tooltip>
    );
    await userEvent.tab();
    expect(screen.queryByRole('tooltip')).toBeNull();
  });
});

describe('disabled with a reason', () => {
  beforeEach(() => wait(SKIP_DELAY + 50));

  it('keeps an icon button focusable, says why and ignores presses', async () => {
    const onClick = vi.fn();
    render(
      <IconButton
        icon="arrow-u-up-left"
        label="Undo"
        shortcut={['⌘', 'Z']}
        disabled
        disabledReason="Nothing to undo yet"
        onClick={onClick}
      />
    );
    const undo = screen.getByRole('button', { name: 'Undo' });
    expect(undo).toHaveAttribute('aria-disabled', 'true');
    expect(undo).not.toBeDisabled();
    await userEvent.tab();
    expect(undo).toHaveFocus();
    const tip = screen.getByRole('tooltip');
    expect(tip).toHaveTextContent('Undo');
    expect(tip).toHaveTextContent('Nothing to undo yet');
    expect(undo).toHaveAccessibleDescription(/Nothing to undo yet/);
    await userEvent.click(undo);
    expect(onClick).not.toHaveBeenCalled();
  });

  it('explains a disabled button, and a loading one blocks for real', async () => {
    const onClick = vi.fn();
    const { rerender } = render(
      <Button disabled disabledReason="Add an object first" onClick={onClick}>
        Make refs
      </Button>
    );
    const make = screen.getByRole('button', { name: 'Make refs' });
    await userEvent.tab();
    expect(screen.getByRole('tooltip')).toHaveTextContent('Add an object first');
    await userEvent.click(make);
    expect(onClick).not.toHaveBeenCalled();

    rerender(
      <Button loading disabled disabledReason="Add an object first" onClick={onClick}>
        Make refs
      </Button>
    );
    expect(make).toBeDisabled();
    expect(make).toHaveAttribute('aria-busy', 'true');
  });

  it('a plain disabled button stays natively disabled', () => {
    render(<IconButton icon="x" label="Close" disabled />);
    expect(screen.getByRole('button', { name: 'Close' })).toBeDisabled();
  });
});

describe('Kbd', () => {
  it('draws one cap per key', () => {
    const { container } = render(<Kbd keys={['⌘', 'Shift', 'Z']} />);
    const caps = [...container.querySelectorAll('kbd kbd')].map(k => k.textContent);
    expect(caps).toEqual(['⌘', 'Shift', 'Z']);
  });
});
