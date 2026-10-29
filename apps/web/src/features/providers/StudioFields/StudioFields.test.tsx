import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { StudioFields } from '.';

const value = { mesh: 'hd-model', rig: 'biped' } as const;

describe('the Tripo Studio fields', () => {
  it('change the tab and the rig', async () => {
    const onChange = vi.fn();
    render(<StudioFields value={value} onChange={onChange} views={1} />);
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Tripo Studio tab' }),
      'Smart Mesh (P2.0)'
    );
    expect(onChange).toHaveBeenCalledWith({ mesh: 'smart-mesh', rig: 'biped' });
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Rig' }), 'Quadruped');
    expect(onChange).toHaveBeenLastCalledWith({ mesh: 'hd-model', rig: 'quadruped' });
    expect(screen.getByRole('option', { name: 'Multi-legged' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'None' })).toBeInTheDocument();
  });

  it('say multi-view follows the images and name the controls per asset', () => {
    const { rerender } = render(
      <StudioFields value={value} onChange={() => {}} views={1} name="Wolf" />
    );
    expect(screen.getByText(/Image to 3D from the front/)).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Rig for Wolf' })).toBeInTheDocument();
    rerender(<StudioFields value={value} onChange={() => {}} views={3} name="Wolf" />);
    expect(screen.getByText(/Multiview to 3D with 3 views/)).toBeInTheDocument();
  });
});
