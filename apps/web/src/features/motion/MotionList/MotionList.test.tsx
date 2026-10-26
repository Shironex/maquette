import { fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openGameMotion, openMotion } from '@/test/open-motion';
import MotionList from './MotionList';

afterEach(() => clearMocks());

describe('the motion list', () => {
  it('lists the models that move first and filters them', async () => {
    await openMotion();
    render(<MotionList />);
    expect(screen.getByText(/^\d+ of \d+ models move$/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Clay figure/ })).toHaveAttribute(
      'aria-current',
      'true'
    );
    fireEvent.click(screen.getByRole('button', { name: 'Props' }));
    expect(screen.queryByRole('button', { name: /Clay figure/ })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Switch cabinet/ }));
    expect(useMotionStore.getState().asset).toBe('switch-cabinet');
    expect(useMotionStore.getState().tab).toBe('states');
  });

  it('marks a model over the scene budget', async () => {
    await openMotion('over');
    render(<MotionList />);
    expect(screen.getAllByText('Over').length).toBeGreaterThan(0);
  });

  it("marks a game's characters over their folder budget, not a scene budget", async () => {
    await openGameMotion();
    render(<MotionList />);
    const hero = screen.getByRole('button', { name: /Heroine/ });
    expect(within(hero).getByText('Over')).toBeInTheDocument();
    expect(within(hero).getByText('Over').closest('[title]')).toHaveAttribute(
      'title',
      "Over its folder's budget: see the Inspector's Budget tab"
    );
  });
});
