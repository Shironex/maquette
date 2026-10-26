import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import TransitionEditor from './TransitionEditor';
import { curvePath } from './TransitionEditor.hooks';

afterEach(() => clearMocks());

describe('the transition editor', () => {
  it('draws the easing from corner to corner', () => {
    const path = curvePath('linear', 100, 50);
    expect(path.startsWith('M0.0,50.0')).toBe(true);
    expect(path.endsWith('100.0,0.0')).toBe(true);
  });

  it('times the move into the selected state', async () => {
    await openMotion(undefined, 'switch-cabinet');
    render(<TransitionEditor />);
    expect(screen.getByText('Closed, off to Door open')).toBeInTheDocument();
    expect(screen.getByText('Door, 1 of 8')).toBeInTheDocument();
    const duration = screen.getByLabelText('Duration');
    fireEvent.change(duration, { target: { value: '1.2' } });
    fireEvent.blur(duration);
    fireEvent.change(screen.getByLabelText('Easing'), { target: { value: 'linear' } });
    const t = useMotionStore.getState().draft.states['switch-cabinet']!.transitions[0]!;
    expect(t).toMatchObject({ duration: 1.2, easing: 'linear' });
  });

  it('shows nothing on the rest state', async () => {
    await openMotion(undefined, 'switch-cabinet');
    useMotionStore.getState().selectState('closed');
    const { container } = render(<TransitionEditor />);
    expect(container).toBeEmptyDOMElement();
  });
});
