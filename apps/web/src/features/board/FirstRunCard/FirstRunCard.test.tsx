import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { readFlag, writeFlag } from '@/lib/storage';
import { useAppStore } from '@/state/use-app-store';
import FirstRunCard from './FirstRunCard';

const TIPS_KEY = 'maquette.board.tipsHidden';

afterEach(() => {
  writeFlag(TIPS_KEY, false);
  useAppStore.setState({ view: 'board' });
});

describe('FirstRunCard', () => {
  it('teaches three steps with the scene budget', () => {
    render(<FirstRunCard name="Moon Island" budget={3} />);
    const card = screen.getByRole('region', { name: 'Your first build in three steps' });
    expect(card).toHaveTextContent('Moon Island is empty.');
    expect(screen.getAllByRole('listitem')).toHaveLength(3);
    expect(card).toHaveTextContent('fits it into the 3.0 MB budget');
  });

  it('opens the Lab', async () => {
    render(<FirstRunCard name="Moon Island" budget={3} />);
    await userEvent.click(screen.getByRole('button', { name: 'Open Concept lab' }));
    expect(useAppStore.getState().view).toBe('lab');
  });

  it('hides the tips for good', async () => {
    const { rerender } = render(<FirstRunCard name="Moon Island" budget={3} />);
    await userEvent.click(screen.getByRole('button', { name: 'Hide tips' }));
    expect(screen.queryByRole('region')).toBeNull();
    expect(readFlag(TIPS_KEY)).toBe(true);
    rerender(<FirstRunCard name="Moon Island" budget={3} />);
    expect(screen.queryByRole('region')).toBeNull();
  });
});
