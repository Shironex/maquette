import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import PoseEditor from './PoseEditor';

afterEach(() => clearMocks());

const cabinet = () => useMotionStore.getState().draft.states['switch-cabinet']!;

describe('the pose editor', () => {
  it('turns a part, renames the state and removes it', async () => {
    await openMotion(undefined, 'switch-cabinet');
    render(<PoseEditor />);
    expect(screen.getByLabelText('Part')).toHaveValue('Door');
    const turn = screen.getByLabelText('Turn Y');
    fireEvent.change(turn, { target: { value: '-90' } });
    fireEvent.blur(turn);
    expect(cabinet().states[1]!.parts.Door!.rotation).toEqual([0, -90, 0]);
    const name = screen.getByLabelText('Name');
    fireEvent.change(name, { target: { value: 'Door ajar' } });
    fireEvent.blur(name);
    expect(cabinet().states[1]!.name).toBe('Door ajar');
    fireEvent.click(screen.getByRole('button', { name: 'Remove state' }));
    expect(cabinet().states.map(s => s.id)).toEqual(['closed', 'breakers-on']);
  });

  it('keeps the rest state at rest', async () => {
    await openMotion(undefined, 'switch-cabinet');
    useMotionStore.getState().selectState('closed');
    render(<PoseEditor />);
    expect(screen.getByText(/every part at rest/)).toBeInTheDocument();
    expect(screen.queryByLabelText('Part')).not.toBeInTheDocument();
  });
});
