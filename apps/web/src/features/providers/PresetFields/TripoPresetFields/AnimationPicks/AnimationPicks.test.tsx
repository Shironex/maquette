import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { AnimationPicks } from '.';
import { tooltipText } from '@/test/tooltipText';

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

  it('stop at five, and say why', async () => {
    const five = ['idle', 'walk', 'run', 'jump', 'wave'];
    render(<AnimationPicks options={options} value={five} onChange={() => {}} />);
    const dance = screen.getByRole('button', { name: 'dance' });
    expect(dance).toHaveAttribute('aria-disabled', 'true');
    expect(await tooltipText(dance)).toBe('Tripo retargets at most 5 at once');
    expect(screen.getByRole('button', { name: 'wave' })).toBeEnabled();
  });
});
