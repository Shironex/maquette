import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import SplitHandle from './SplitHandle';

function inBox(onChange: (split: number) => void, split = 0.5) {
  render(
    <div data-testid="box">
      <SplitHandle split={split} onChange={onChange} />
    </div>
  );
  screen.getByTestId('box').getBoundingClientRect = () => new DOMRect(100, 0, 400, 300);
  return screen.getByRole('slider', { name: 'Compare position' });
}

describe('the split handle', () => {
  it('stands at the split and says where it is', () => {
    const handle = inBox(() => {}, 0.3);
    expect(handle).toHaveAttribute('aria-valuenow', '30');
    expect(handle).toHaveStyle({ left: '30%' });
  });

  it('moves in steps with the arrow keys, never past the edges', async () => {
    const onChange = vi.fn();
    const handle = inBox(onChange, 0.04);
    handle.focus();
    await userEvent.keyboard('{ArrowLeft}');
    expect(onChange).toHaveBeenLastCalledWith(0.02);
    await userEvent.keyboard('{ArrowRight}');
    expect(onChange.mock.lastCall?.[0]).toBeCloseTo(0.09);
  });

  it('follows a drag across the view, held back from the edges', () => {
    const onChange = vi.fn();
    const handle = inBox(onChange);
    fireEvent.pointerMove(handle, { clientX: 200, buttons: 1 });
    expect(onChange).toHaveBeenLastCalledWith(0.25);
    fireEvent.pointerMove(handle, { clientX: 900, buttons: 1 });
    expect(onChange).toHaveBeenLastCalledWith(0.98);
    fireEvent.pointerMove(handle, { clientX: 300, buttons: 0 });
    expect(onChange).toHaveBeenCalledTimes(2);
  });
});
