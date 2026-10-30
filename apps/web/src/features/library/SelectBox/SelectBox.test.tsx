import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import SelectBox from './SelectBox';

describe('a model select box', () => {
  it('is a checkbox named for the model that passes Shift on as a range', async () => {
    const onCheck = vi.fn();
    render(<SelectBox name="Street lamp" checked={false} selecting={false} onCheck={onCheck} />);
    const box = screen.getByRole('checkbox', { name: 'Select Street lamp' });
    expect(box).not.toBeChecked();
    await userEvent.click(box);
    expect(onCheck).toHaveBeenLastCalledWith(true, false);
    const user = userEvent.setup();
    await user.keyboard('{Shift>}');
    await user.click(box);
    expect(onCheck).toHaveBeenLastCalledWith(true, true);
  });

  it('unchecks when it is checked', async () => {
    const onCheck = vi.fn();
    render(<SelectBox name="Bee hive" checked selecting onCheck={onCheck} />);
    const box = screen.getByRole('checkbox', { name: 'Select Bee hive' });
    expect(box).toBeChecked();
    await userEvent.click(box);
    expect(onCheck).toHaveBeenLastCalledWith(false, false);
  });
});
