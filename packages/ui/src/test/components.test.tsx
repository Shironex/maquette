import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import {
  AssetCard,
  LogConsole,
  PipelineNode,
  budgetScale,
  NumberField,
  SegmentedControl,
  StageStepper,
  TextArea,
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

describe('AssetCard', () => {
  it('opens its menu from the dots button', async () => {
    const onMore = vi.fn();
    const { rerender } = render(
      <AssetCard
        name="Island"
        version={7}
        status="done"
        onMore={onMore}
        menu={<div role="menu">Move</div>}
      />
    );
    const dots = screen.getByRole('button', { name: 'More for Island' });
    expect(dots).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(dots);
    expect(onMore).toHaveBeenCalledOnce();
    expect(screen.queryByRole('menu')).toBeNull();
    rerender(
      <AssetCard
        name="Island"
        version={7}
        status="done"
        onMore={onMore}
        moreOpen
        menu={<div role="menu">Move</div>}
      />
    );
    expect(screen.getByRole('menu')).toHaveTextContent('Move');
  });
});

describe('PipelineNode', () => {
  it('shows a custom chip label and actions', () => {
    render(
      <PipelineNode
        step={5}
        name="Capture posters"
        state="waiting"
        label="failed in 24"
        actions={<button type="button">Run from here</button>}
      />
    );
    expect(screen.getByText('failed in 24')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Run from here' })).toBeInTheDocument();
  });

  it('draws a known fraction while running, and an indeterminate bar otherwise', () => {
    const { rerender } = render(
      <PipelineNode step={2} name="Bake light" state="running" progress={68} />
    );
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '68');
    rerender(<PipelineNode step={2} name="Bake light" state="running" />);
    const bar = screen.getByRole('progressbar');
    expect(bar).not.toHaveAttribute('aria-valuenow');
    expect(bar).toHaveAttribute('aria-busy', 'true');
    rerender(<PipelineNode step={2} name="Bake light" state="done" />);
    expect(screen.queryByRole('progressbar')).toBeNull();
  });
});

describe('LogConsole', () => {
  it('shows the empty text and keeps the newest line in view', () => {
    const { rerender } = render(
      <LogConsole title="Log" lines={[]} empty="No output yet" autoScroll />
    );
    expect(screen.getByRole('log')).toHaveTextContent('No output yet');
    const log = screen.getByRole('log');
    Object.defineProperty(log, 'scrollHeight', { value: 900, configurable: true });
    rerender(
      <LogConsole
        title="Log"
        lines={[{ t: '15:02:31', text: 'Blender quit' }]}
        empty="No output yet"
        autoScroll
      />
    );
    expect(log.scrollTop).toBe(900);
    expect(log).not.toHaveTextContent('No output yet');
  });
});

describe('TextArea', () => {
  it('labels the field and passes typing through', async () => {
    const onChange = vi.fn();
    render(<TextArea label="Prompt" hint="What the concepts show" onChange={onChange} />);
    const box = screen.getByLabelText('Prompt');
    expect(box.tagName).toBe('TEXTAREA');
    await userEvent.type(box, 'moon');
    expect(onChange).toHaveBeenCalledTimes(4);
  });
});
