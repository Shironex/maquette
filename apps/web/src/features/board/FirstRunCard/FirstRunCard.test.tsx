import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { readFlag, writeFlag } from '@/lib/storage';
import { useAppStore } from '@/state/use-app-store';
import { useGameStore } from '@/state/use-game-store';
import { clearMocks, openMockGame } from '@/test/mock-ipc';
import FirstRunCard from './FirstRunCard';

const TIPS_KEY = 'maquette.board.tipsHidden';

afterEach(() => {
  clearMocks();
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

  it('teaches a game its folder budgets and imports from the game repo', async () => {
    await openMockGame('import');
    render(<FirstRunCard name="Crypt game" budget={3} />);
    const card = screen.getByRole('region', { name: 'Your first build in three steps' });
    expect(card).not.toHaveTextContent('MB');
    expect(card).not.toHaveTextContent('scene');
    expect(card).toHaveTextContent("at its folder's budget");
    await userEvent.click(screen.getByRole('button', { name: 'Import models' }));
    expect(useGameStore.getState().importFor).toBe('crypt-game');
    useGameStore.setState({ importFor: null, scan: null, picked: [] });
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
