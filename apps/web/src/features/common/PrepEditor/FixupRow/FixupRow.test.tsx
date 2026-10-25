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
});
