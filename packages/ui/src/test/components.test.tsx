import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import {
  budgetScale,
  NumberField,
  SegmentedControl,
  StageStepper,
  Toggle,
  ToolCheckRow,
} from '../index';

describe('budgetScale', () => {
  it('parks the target at 80 percent while under budget', () => {
    expect(budgetScale(1.5, 3).targetAt).toBeCloseTo(80);
    expect(budgetScale(1.5, 3).over).toBe(false);
  });

  it('fills the whole bar once over budget', () => {
    const s = budgetScale(5.1, 3);
    expect(s.over).toBe(true);
    expect(s.fill).toBe(100);
  });

  it('copes with an empty project', () => {
    expect(budgetScale(0, 3).fill).toBe(0);
  });
});

describe('NumberField', () => {
  it('commits a clamped number on blur', async () => {
    const onChange = vi.fn();
    render(<NumberField label="Tris per model" value={60000} onChange={onChange} min={1000} />);
    const input = screen.getByLabelText('Tris per model');
    await userEvent.clear(input);
    await userEvent.type(input, '12');
    await userEvent.tab();
    expect(onChange).toHaveBeenCalledWith(1000);
  });

  it('reads thousands separators and reverts text it cannot read', async () => {
    const onChange = vi.fn();
    render(<NumberField label="Quads" value={50000} onChange={onChange} />);
    const input = screen.getByLabelText('Quads');
    await userEvent.clear(input);
    await userEvent.type(input, '42,000{Enter}');
    expect(onChange).toHaveBeenLastCalledWith(42000);
    await userEvent.clear(input);
    await userEvent.type(input, 'lots');
    await userEvent.tab();
    expect(input).toHaveValue('50000');
  });
});

describe('SegmentedControl', () => {
  it('marks the chosen option pressed', async () => {
    const onChange = vi.fn();
    render(
      <SegmentedControl
        label="Texture codec"
        options={[
          { value: 'etc1s', label: 'ETC1S' },
          { value: 'uastc', label: 'UASTC' },
        ]}
        value="etc1s"
        onChange={onChange}
      />
    );
    expect(screen.getByRole('button', { name: 'ETC1S' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'UASTC' }));
    expect(onChange).toHaveBeenCalledWith('uastc');
  });
});

describe('Toggle', () => {
  it('is a switch with an accessible name', async () => {
    const onChange = vi.fn();
    render(<Toggle aria-label="Ask before shipping" checked onChange={onChange} />);
    const sw = screen.getByRole('switch', { name: 'Ask before shipping' });
    expect(sw).toBeChecked();
    await userEvent.click(sw);
    expect(onChange).toHaveBeenCalledWith(false);
  });
});

describe('ToolCheckRow', () => {
  it('reads a missing required tool as failed and an optional one as a warning', () => {
    const { rerender } = render(<ToolCheckRow name="Blender" required status="missing" />);
    expect(screen.getByRole('img', { name: 'Missing' })).toHaveClass('text-danger');
    rerender(<ToolCheckRow name="ffmpeg" status="missing" />);
    expect(screen.getByRole('img', { name: 'Missing' })).toHaveClass('text-amber');
  });
});

describe('StageStepper', () => {
  it('says each stage state in words, not only colour', () => {
    render(
      <StageStepper
        stages={[
          { id: 'concept', label: 'Concept', state: 'done' },
          { id: 'refs', label: 'Refs', state: 'current' },
        ]}
      />
    );
    expect(screen.getByText(', done')).toBeInTheDocument();
    expect(screen.getByText('Refs').closest('li')).toHaveAttribute('aria-current', 'step');
  });
});
