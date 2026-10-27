import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { gameProject } from '@/ipc/mock/game-fixtures';
import { budgetSummary } from '@/state/game';
import { ModelBudgetMeter } from '.';

describe('ModelBudgetMeter', () => {
  it('counts the models over their folder budget', () => {
    render(<ModelBudgetMeter summary={budgetSummary(gameProject('crypt', true))!} />);
    expect(
      screen.getByRole('meter', { name: 'Folder budgets, 3 of 15 models over' })
    ).toHaveAttribute('aria-valuenow', '12');
    expect(screen.getByText('3 over')).toBeInTheDocument();
  });

  it('shows none over when every model fits', () => {
    render(
      <ModelBudgetMeter summary={{ models: 1, over: [], overTriangles: 0, overTextures: 0 }} />
    );
    expect(
      screen.getByRole('meter', { name: 'Folder budgets, 0 of 1 models over' })
    ).toBeInTheDocument();
  });
});
