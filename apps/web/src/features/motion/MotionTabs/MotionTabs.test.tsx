import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import MotionTabs from './MotionTabs';

afterEach(() => useMotionStore.setState({ tab: 'clips' }));

describe('the motion tabs', () => {
  it('switches between clips and states', () => {
    render(<MotionTabs />);
    fireEvent.click(screen.getByRole('tab', { name: 'States' }));
    expect(useMotionStore.getState().tab).toBe('states');
  });
});
