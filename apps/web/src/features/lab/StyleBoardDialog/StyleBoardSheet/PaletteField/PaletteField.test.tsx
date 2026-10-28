import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PaletteField } from '.';

afterEach(cleanup);

describe('the palette field', () => {
  it('adds typed colours as #rrggbb and removes one', async () => {
    const onChange = vi.fn();
    render(<PaletteField palette={['#1b2440']} onChange={onChange} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Colour' }), 'ABC{Enter}');
    expect(onChange).toHaveBeenLastCalledWith(['#1b2440', '#aabbcc']);
    await userEvent.click(screen.getByRole('button', { name: 'Remove #1b2440' }));
    expect(onChange).toHaveBeenLastCalledWith([]);
    expect(screen.getByText('1 of 6')).toBeVisible();
  });

  it('says how to write a colour it cannot read', async () => {
    const onChange = vi.fn();
    render(<PaletteField palette={[]} onChange={onChange} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Colour' }), 'frost');
    await userEvent.click(screen.getByRole('button', { name: 'Add' }));
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByText('Write a colour as #rrggbb, or #rgb.')).toBeVisible();
  });

  it('hides the field once the palette is full', () => {
    const six = ['#111111', '#222222', '#333333', '#444444', '#555555', '#666666'];
    render(<PaletteField palette={six} onChange={() => {}} />);
    expect(screen.queryByRole('textbox', { name: 'Colour' })).toBeNull();
    expect(screen.getByText('6 of 6')).toBeVisible();
  });
});
