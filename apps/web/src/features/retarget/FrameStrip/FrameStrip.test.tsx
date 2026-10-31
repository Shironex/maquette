import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import FrameStrip from './FrameStrip';

describe('the frame strip', () => {
  it('moves a trim handle by the arrow keys and keeps in before out', () => {
    const onTrim = vi.fn();
    render(<FrameStrip frames={40} label="Trim" range={[0, 1]} onTrim={onTrim} />);
    fireEvent.keyDown(screen.getByRole('slider', { name: 'Trim out' }), { key: 'ArrowLeft' });
    expect(onTrim).toHaveBeenCalledWith([0, 1]);
    fireEvent.keyDown(screen.getByRole('slider', { name: 'Trim in' }), { key: 'ArrowRight' });
    expect(onTrim).toHaveBeenLastCalledWith([0, 1]);
  });

  it('names the flagged range and scrubs by the keys', () => {
    const onSeek = vi.fn();
    render(
      <FrameStrip
        frames={22}
        label="Frame"
        frame={3}
        onSeek={onSeek}
        flagged={[9, 14]}
        flaggedLabel="Slides 4.2 cm, frames 9 to 14"
      />
    );
    expect(screen.getByText('Slides 4.2 cm, frames 9 to 14')).toBeInTheDocument();
    fireEvent.keyDown(screen.getByRole('slider', { name: 'Frame' }), { key: 'ArrowRight' });
    expect(onSeek).toHaveBeenCalledWith(4);
  });
});
