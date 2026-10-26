import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import TriggerList from './TriggerList';

afterEach(() => clearMocks());

const triggers = () => useMotionStore.getState().draft.states['switch-cabinet']!.triggers;

describe('the trigger list', () => {
  it('lists the triggers, adds a hover and removes one', async () => {
    await openMotion(undefined, 'switch-cabinet');
    render(<TriggerList />);
    expect(screen.getByText('Triggers · 3')).toBeInTheDocument();
    expect(screen.getByText('Click on Breaker 1')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));
    fireEvent.click(screen.getByRole('menuitem', { name: 'A hover on the model' }));
    expect(triggers().at(-1)).toMatchObject({ kind: 'hover', to: 'door-open' });
    fireEvent.click(screen.getByRole('button', { name: 'Remove Click on Breaker 1' }));
    expect(triggers().some(t => t.kind === 'click')).toBe(false);
  });
});
