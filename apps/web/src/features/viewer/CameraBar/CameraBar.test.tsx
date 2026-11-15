import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import CameraBar from './CameraBar';

describe('the camera bar', () => {
  it('turns following on and off, and lists F to frame the model', async () => {
    const onFollow = vi.fn();
    const { rerender } = render(<CameraBar follow onFollow={onFollow} />);
    const button = screen.getByRole('button', { name: 'Stop following the model' });
    expect(button).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(button);
    expect(onFollow).toHaveBeenCalledWith(false);
    rerender(<CameraBar follow={false} onFollow={onFollow} />);
    expect(screen.getByRole('button', { name: 'Follow the model' })).toHaveAttribute(
      'aria-pressed',
      'false'
    );
    expect(screen.getByRole('toolbar', { name: 'Camera' })).toHaveTextContent('Fframe');
  });
});
