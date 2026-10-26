import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import StatesStage from './StatesStage';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

afterEach(() => clearMocks());

describe('the states stage', () => {
  it('shows the selected state at the end of the move into it, and scrubs it', async () => {
    await openMotion(undefined, 'switch-cabinet');
    render(<StatesStage />);
    expect(screen.getByLabelText('State')).toHaveValue('door-open');
    expect(screen.getByText('Door open · end of the 0.6 s move')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Scrub'), { target: { value: '50' } });
    expect(screen.getByText('Door open · 50% of the 0.6 s move')).toBeInTheDocument();
  });

  it('offers states to a model with none', async () => {
    await openMotion(undefined, 'turntable');
    useMotionStore.getState().setTab('states');
    render(<StatesStage />);
    fireEvent.click(screen.getByRole('button', { name: 'Give it states' }));
    expect(useMotionStore.getState().draft.states.turntable?.states).toHaveLength(1);
  });
});
