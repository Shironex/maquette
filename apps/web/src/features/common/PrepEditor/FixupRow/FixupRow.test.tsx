import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { FixupRow } from '.';

describe('FixupRow', () => {
  it('edits the number a fix-up takes and removes it', async () => {
    const onChange = vi.fn();
    const onRemove = vi.fn();
    render(
      <ul>
        <FixupRow
          fixup={{ op: 'shrink-textures', maxSize: 2048 }}
          index={2}
          onChange={onChange}
          onRemove={onRemove}
        />
      </ul>
    );
    await userEvent.selectOptions(screen.getByLabelText('Largest texture'), '1024');
    expect(onChange).toHaveBeenCalledWith({ op: 'shrink-textures', maxSize: 1024 });
    await userEvent.click(screen.getByRole('button', { name: 'Remove shrink textures' }));
    expect(onRemove).toHaveBeenCalled();
  });

  it('sets the triangle target and the merge mode', async () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <ul>
        <FixupRow
          fixup={{ op: 'decimate', triangles: 60_000 }}
          index={1}
          onChange={onChange}
          onRemove={vi.fn()}
        />
      </ul>
    );
    const field = screen.getByLabelText('Triangle target');
    await userEvent.clear(field);
    await userEvent.type(field, '5000{Enter}');
    expect(onChange).toHaveBeenLastCalledWith({ op: 'decimate', triangles: 5000 });
    rerender(
      <ul>
        <FixupRow
          fixup={{ op: 'merge', byMaterial: false }}
          index={1}
          onChange={onChange}
          onRemove={vi.fn()}
        />
      </ul>
    );
    await userEvent.click(screen.getByRole('button', { name: 'By material' }));
    expect(onChange).toHaveBeenLastCalledWith({ op: 'merge', byMaterial: true });
  });

  it('splits by loose pieces or material and merges the tiny bits', async () => {
    const onChange = vi.fn();
    render(
      <ul>
        <FixupRow
          fixup={{ op: 'split-parts', by: 'loose', mergeUnder: 0.01 }}
          index={3}
          onChange={onChange}
          onRemove={vi.fn()}
        />
      </ul>
    );
    expect(screen.getByText('Split into parts')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'By material' }));
    expect(onChange).toHaveBeenLastCalledWith({
      op: 'split-parts',
      by: 'material',
      mergeUnder: 0.01,
    });
    const field = screen.getByLabelText('Merge bits under');
    await userEvent.clear(field);
    await userEvent.type(field, '3{Enter}');
    expect(onChange).toHaveBeenLastCalledWith({ op: 'split-parts', by: 'loose', mergeUnder: 0.03 });
  });

  it('turns a leaning model by degrees about each axis', async () => {
    const onChange = vi.fn();
    render(
      <ul>
        <FixupRow
          fixup={{ op: 'rotate', x: 0, y: 0, z: 0 }}
          index={1}
          onChange={onChange}
          onRemove={vi.fn()}
        />
      </ul>
    );
    expect(screen.getByText('Rotate')).toBeInTheDocument();
    const field = screen.getByLabelText('Degrees about x');
    await userEvent.clear(field);
    await userEvent.type(field, '-24.5{Enter}');
    expect(onChange).toHaveBeenLastCalledWith({ op: 'rotate', x: -24.5, y: 0, z: 0 });
    expect(screen.getByLabelText('Degrees about z')).toBeInTheDocument();
  });
});
