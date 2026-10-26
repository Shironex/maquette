import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import StatesStrip from './StatesStrip';

afterEach(() => clearMocks());

describe('the states strip', () => {
  it('shows each state with the move into it and adds one', async () => {
    await openMotion(undefined, 'switch-cabinet');
    render(<StatesStrip />);
    expect(screen.getByText('States · 3')).toBeInTheDocument();
    expect(screen.getByText('rest pose')).toBeInTheDocument();
    expect(screen.getByText('7 parts moved')).toBeInTheDocument();
    expect(screen.getByText('stagger')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Closed, off/ }));
    expect(useMotionStore.getState().state).toBe('closed');
    fireEvent.click(screen.getByRole('button', { name: 'Add state' }));
    const s = useMotionStore.getState();
    expect(s.draft.states['switch-cabinet']?.states).toHaveLength(4);
    expect(s.state).toBe('state-4');
  });
});
