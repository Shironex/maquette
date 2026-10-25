import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { SizeRow } from '.';

describe('SizeRow', () => {
  it('keeps the length when the unit changes and edits the side', async () => {
    const onChange = vi.fn();
    render(
      <SizeRow
        size={{ axis: 'height', value: 45, unit: 'cm' }}
        onChange={onChange}
        onClear={vi.fn()}
      />
    );
    await userEvent.click(screen.getByRole('button', { name: 'm' }));
    expect(onChange).toHaveBeenLastCalledWith({ axis: 'height', value: 0.45, unit: 'm' });
    await userEvent.selectOptions(screen.getByLabelText('Side'), 'width');
    expect(onChange).toHaveBeenLastCalledWith({ axis: 'width', value: 45, unit: 'cm' });
  });

  it('clears the size', async () => {
    const onClear = vi.fn();
    render(
      <SizeRow
        size={{ axis: 'longest', value: 1.2, unit: 'm' }}
        onChange={vi.fn()}
        onClear={onClear}
      />
    );
    await userEvent.click(screen.getByRole('button', { name: 'Clear the real size' }));
    expect(onClear).toHaveBeenCalled();
  });
});
