import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import PresetPicker from './PresetPicker';

describe('PresetPicker', () => {
  it('offers the three presets as one radio group, with their tags', async () => {
    const onChange = vi.fn();
    render(<PresetPicker value="brainstorm" onChange={onChange} />);
    const group = screen.getByRole('radiogroup', { name: 'Start from' });
    expect(group).toHaveTextContent('Default');
    expect(group).toHaveTextContent('For games');
    expect(screen.getByRole('radio', { name: /^Brainstorm/ })).toBeChecked();
    expect(screen.getByRole('radio', { name: /^Story/ })).not.toBeChecked();
    await userEvent.click(screen.getByRole('radio', { name: /^Story/ }));
    expect(onChange).toHaveBeenCalledWith('story');
    await userEvent.keyboard('{ArrowDown}');
    expect(onChange).toHaveBeenLastCalledWith('mechanics');
  });

  it('takes another label', () => {
    render(<PresetPicker value="story" onChange={vi.fn()} label="Preset" />);
    expect(screen.getByRole('radiogroup', { name: 'Preset' })).toBeInTheDocument();
  });
});
