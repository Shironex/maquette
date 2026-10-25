import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { AnimationPicks } from '.';

const options = ['idle', 'walk', 'run', 'jump', 'wave', 'dance'].map(id => ({
  id,
  label: id,
  note: '',
}));

describe('the animation picks', () => {
  it('add and remove a pick', async () => {
    const onChange = vi.fn();
    render(<AnimationPicks options={options} value={['idle']} onChange={onChange} />);
    await userEvent.click(screen.getByRole('button', { name: 'walk' }));
    expect(onChange).toHaveBeenLastCalledWith(['idle', 'walk']);
    await userEvent.click(screen.getByRole('button', { name: 'idle' }));
    expect(onChange).toHaveBeenLastCalledWith([]);
  });

  it('stop at five', () => {
    const five = ['idle', 'walk', 'run', 'jump', 'wave'];
    render(<AnimationPicks options={options} value={five} onChange={() => {}} />);
    expect(screen.getByRole('button', { name: 'dance' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'wave' })).toBeEnabled();
  });
});
