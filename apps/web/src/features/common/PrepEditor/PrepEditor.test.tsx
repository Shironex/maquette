import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { ModelPrep } from '@/ipc/client';
import { PrepEditor } from '.';
import type { PrepEditorProps } from './PrepEditor.types';

const defaults = { triangles: 60_000, maxTexture: 2048, looseShare: 0.02 };

function setup(over: Partial<PrepEditorProps> = {}) {
  const onChange = vi.fn<(next: ModelPrep) => void>();
  const onPreview = vi.fn();
  render(
    <PrepEditor
      value={{ size: null, fixups: [] }}
      onChange={onChange}
      measured={[1.25, 0.7, 0.5]}
      suggestion={{ size: { axis: 'height', value: 45, unit: 'cm' }, matched: 'lantern post' }}
      defaults={defaults}
      preview={null}
      onPreview={onPreview}
      {...over}
    />
  );
  return { onChange, onPreview };
}

describe('PrepEditor', () => {
  it('offers the suggestion without applying it, and starts a size from the measured height', async () => {
    const { onChange } = setup();
    expect(screen.getByText('Measured 1.25 × 0.7 × 0.5 m')).toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: /Suggested: 45 cm high/ }));
    expect(onChange).toHaveBeenLastCalledWith({
      size: { axis: 'height', value: 45, unit: 'cm' },
      fixups: [],
    });
    await userEvent.click(screen.getByRole('button', { name: 'Set real size' }));
    expect(onChange).toHaveBeenLastCalledWith({
      size: { axis: 'height', value: 70, unit: 'cm' },
      fixups: [],
    });
  });

  it('shows the scale a size gives and hides a suggestion already taken', () => {
    setup({ value: { size: { axis: 'height', value: 45, unit: 'cm' }, fixups: [] } });
    expect(screen.getByText('Scaled ×0.643, so 80.4 × 45 × 32.1 cm')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Suggested/ })).toBeNull();
  });

  it('adds fix-ups from the workspace budgets in the order they apply', async () => {
    const { onChange } = setup({
      value: { size: null, fixups: [{ op: 'origin-bottom' }] },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Decimate' }));
    expect(onChange).toHaveBeenLastCalledWith({
      size: null,
      fixups: [{ op: 'decimate', triangles: 60_000 }, { op: 'origin-bottom' }],
    });
    expect(screen.queryByRole('button', { name: 'Origin to bottom centre' })).toBeNull();
  });

  it('runs a preview only once there is something to run', async () => {
    const empty = setup();
    const button = screen.getByRole('button', { name: 'Preview the numbers' });
    expect(button).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(button);
    expect(empty.onPreview).not.toHaveBeenCalled();
  });

  it('says why a preview failed', () => {
    setup({
      value: { size: null, fixups: [{ op: 'merge', byMaterial: false }] },
      preview: { status: 'failed', error: 'Blender is not installed.' },
    });
    expect(screen.getByRole('alert')).toHaveTextContent('Blender is not installed.');
  });

  it('switches the bake off and on, starting from the Inbox default', async () => {
    const { onChange } = setup({ inboxDefaults: true });
    const toggle = screen.getByRole('switch', { name: /Skip the bake/ });
    expect(toggle).toBeChecked();
    await userEvent.click(toggle);
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ skipBake: false }));
  });

  it('pads texture edges by default for an Inbox model only, and switches per asset', async () => {
    const { onChange } = setup({ inboxDefaults: true });
    const toggle = screen.getByRole('switch', { name: /Pad texture edges/ });
    expect(toggle).toBeChecked();
    await userEvent.click(toggle);
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ padEdges: false }));
  });

  it('leaves texture edges alone for a model nobody chose for outside the Inbox', async () => {
    const { onChange } = setup({ value: { size: null, fixups: [], padEdges: null } });
    const toggle = screen.getByRole('switch', { name: /Pad texture edges/ });
    expect(toggle).not.toBeChecked();
    await userEvent.click(toggle);
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ padEdges: true }));
  });
});
