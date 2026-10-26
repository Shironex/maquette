import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import ClipTimeline from './ClipTimeline';

afterEach(() => clearMocks());

describe('the clip timeline', () => {
  it('plays, steps and sets the preview loop from the keyboard', async () => {
    await openMotion();
    useMotionStore.getState().setPlayer({ playing: false });
    render(<ClipTimeline />);
    fireEvent.keyDown(window, { key: ' ' });
    expect(useMotionStore.getState().player.playing).toBe(true);
    fireEvent.keyDown(window, { key: '.' });
    fireEvent.keyDown(window, { key: '.' });
    expect(screen.getByRole('slider', { name: 'Playhead' })).toHaveAttribute('aria-valuenow', '2');
    fireEvent.keyDown(window, { key: 'i' });
    expect(useMotionStore.getState().player.range[0]).toBe(2);
    fireEvent.keyDown(window, { key: 'l' });
    expect(useMotionStore.getState().player.loop).toBe(false);
  });

  it('changes the speed', async () => {
    await openMotion();
    render(<ClipTimeline />);
    fireEvent.change(screen.getByLabelText('Speed'), { target: { value: '0.5' } });
    expect(useMotionStore.getState().player.speed).toBe('0.5');
  });
});
