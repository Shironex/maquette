import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import RecipeRange from './RecipeRange';

describe('the recipe range', () => {
  it('shows the range in frames and moves a handle a frame with the arrow keys', () => {
    const onChange = vi.fn();
    render(<RecipeRange frames={100} start={0.2} end={0.6} onChange={onChange} />);
    expect(screen.getByLabelText('Start frame')).toHaveValue('20');
    fireEvent.keyDown(screen.getByRole('button', { name: 'Ends at frame 60' }), {
      key: 'ArrowRight',
    });
    expect(onChange).toHaveBeenLastCalledWith(0.2, 0.61);
  });

  it('keeps the start before the end when a frame is typed past it', async () => {
    const onChange = vi.fn();
    render(<RecipeRange frames={100} start={0.2} end={0.6} onChange={onChange} />);
    const start = screen.getByLabelText('Start frame');
    await userEvent.clear(start);
    await userEvent.type(start, '90{Enter}');
    expect(onChange).toHaveBeenLastCalledWith(0.59, 0.6);
  });
});
