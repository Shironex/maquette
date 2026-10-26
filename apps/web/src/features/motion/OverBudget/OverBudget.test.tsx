import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import OverBudget from './OverBudget';

afterEach(() => clearMocks());

describe('the over budget board', () => {
  it('shows nothing while the scene fits', async () => {
    await openMotion();
    const { container } = render(<OverBudget />);
    expect(container).toBeEmptyDOMElement();
  });

  it('says how far over the scene is and leaves the heaviest clip off', async () => {
    await openMotion('over');
    render(<OverBudget />);
    expect(screen.getByRole('alert')).toHaveTextContent(/over its 2.25 MB budget/);
    expect(screen.getByText('Keep sitting_wave off the site')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open Pipeline' })).toBeInTheDocument();
    for (const button of screen.getAllByRole('button', { name: 'Leave it off' })) {
      fireEvent.click(button);
    }
    expect(useMotionStore.getState().draft.clipsOff.character).toEqual(['sitting_wave']);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
