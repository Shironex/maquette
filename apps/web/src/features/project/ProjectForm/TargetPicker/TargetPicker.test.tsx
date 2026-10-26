import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { TargetPicker } from '.';

describe('TargetPicker', () => {
  it('marks the target and switches to Game', async () => {
    const onChange = vi.fn();
    render(<TargetPicker value="website" onChange={onChange} />);
    expect(screen.getByRole('button', { name: /Website/ })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(screen.getByRole('button', { name: /Game/ }));
    expect(onChange).toHaveBeenCalledWith('game');
  });
});
