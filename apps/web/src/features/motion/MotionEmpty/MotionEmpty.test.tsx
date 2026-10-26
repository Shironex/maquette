import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import MotionEmpty from './MotionEmpty';

afterEach(() => clearMocks());

describe('the empty motion view', () => {
  it('names the project and suggests the prop with the most parts', async () => {
    await openMotion('still');
    render(<MotionEmpty />);
    expect(screen.getByRole('heading', { name: /^Nothing in .+ moves yet$/ })).toBeInTheDocument();
    expect(screen.getByText('Switch cabinet could move')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Give it states' }));
    expect(useMotionStore.getState().tab).toBe('states');
    expect(useMotionStore.getState().asset).toBe('switch-cabinet');
  });
});
