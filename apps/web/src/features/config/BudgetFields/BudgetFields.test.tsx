import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { BudgetFields } from '.';

describe('the budget fields', () => {
  it('show the budgets and patch the one that changed', async () => {
    const config = defaultConfig();
    const onChange = vi.fn();
    render(<BudgetFields config={config} onChange={onChange} />);
    const tris = screen.getByRole('textbox', { name: 'Tris per model' });
    await userEvent.clear(tris);
    await userEvent.type(tris, '12k');
    fireEvent.blur(tris);
    expect(onChange).toHaveBeenLastCalledWith({
      ...config,
      budgets: { ...config.budgets, trisPerModel: 12_000 },
    });
  });

  it('put an error under its field in place of the hint', () => {
    render(
      <BudgetFields
        config={defaultConfig()}
        onChange={() => {}}
        errors={{ firstViewMb: 'Larger than the scene' }}
      />
    );
    expect(screen.getByText('Larger than the scene')).toBeInTheDocument();
    expect(screen.queryByText('What the first camera stop may load.')).toBeNull();
  });
});
