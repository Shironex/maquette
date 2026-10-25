import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import Vec3Fields from './Vec3Fields';

describe('the three axis fields', () => {
  it('shows each axis with its digits, as one labelled group', () => {
    render(
      <Vec3Fields label="Position" unit="m" digits={3} value={[1, 2, 3.5]} onChange={() => {}} />
    );
    const group = screen.getByRole('group', { name: 'Position in m' });
    expect(within(group).getByLabelText('X')).toHaveValue('1.000');
    expect(within(group).getByLabelText('Y')).toHaveValue('2.000');
    expect(within(group).getByLabelText('Z')).toHaveValue('3.500');
  });

  it('changes one axis and keeps the other two', async () => {
    const onChange = vi.fn();
    render(
      <Vec3Fields
        label="Rotation"
        unit="degrees"
        digits={1}
        value={[0, 90, 0]}
        onChange={onChange}
      />
    );
    const y = screen.getByLabelText('Y');
    await userEvent.clear(y);
    await userEvent.type(y, '45{Enter}');
    expect(onChange).toHaveBeenCalledWith([0, 45, 0]);
  });
});
