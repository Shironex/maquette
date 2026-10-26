import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import PressToggle from './PressToggle';

describe('a press toggle', () => {
  it('says whether it is pressed and flips on a click', () => {
    const onChange = vi.fn();
    render(<PressToggle icon="bone" label="Skeleton" pressed onChange={onChange} />);
    const button = screen.getByRole('button', { name: 'Skeleton' });
    expect(button).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(button);
    expect(onChange).toHaveBeenCalledWith(false);
  });
});
