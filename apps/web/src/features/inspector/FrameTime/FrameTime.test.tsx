import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import FrameTime from './FrameTime';

describe('the frame time card', () => {
  it('says it is measuring before the first sample', () => {
    render(<FrameTime samples={[]} />);
    expect(screen.getByRole('img', { name: 'Frame time, measuring' })).toBeInTheDocument();
    expect(screen.getByText('…')).toBeInTheDocument();
  });

  it('shows the last second under the target in the calm colour', () => {
    render(<FrameTime samples={[10, 12, 12, 12, 12, 12, 12, 12]} />);
    expect(
      screen.getByRole('img', {
        name: 'Frame time over the last 2 seconds, now 12.0 ms, under the 16.7 ms target',
      })
    ).toBeInTheDocument();
    expect(screen.getByText('12.0', { exact: false })).toHaveClass('text-fg');
  });

  it('marks a last second over the target', () => {
    render(<FrameTime samples={[20, 21, 22, 23]} />);
    expect(
      screen.getByRole('img', { name: /now 21\.5 ms, over the 16\.7 ms target/ })
    ).toBeInTheDocument();
    expect(screen.getByText('21.5', { exact: false })).toHaveClass('text-danger');
  });
});
