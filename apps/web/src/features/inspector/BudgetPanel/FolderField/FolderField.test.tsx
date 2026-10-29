import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { FolderField } from '.';

describe('FolderField', () => {
  it('saves a typed folder on Enter and clears it when emptied', async () => {
    const onSave = vi.fn();
    const { rerender } = render(
      <FolderField name="Stone wall" folder={null} fallback="props" onSave={onSave} />
    );
    const field = screen.getByLabelText('Game folder of Stone wall');
    expect(field).toHaveAttribute('placeholder', 'props');
    await userEvent.type(field, 'kit/{Enter}');
    expect(onSave).toHaveBeenLastCalledWith('kit');
    rerender(<FolderField name="Stone wall" folder="kit" fallback="props" onSave={onSave} />);
    await userEvent.clear(field);
    await userEvent.tab();
    expect(onSave).toHaveBeenLastCalledWith(null);
  });

  it('refuses a folder outside the model folder', async () => {
    const onSave = vi.fn();
    render(<FolderField name="Stone wall" folder={null} fallback="" onSave={onSave} />);
    const field = screen.getByLabelText('Game folder of Stone wall');
    expect(field).toHaveAttribute('placeholder', 'top level');
    await userEvent.type(field, '../up{Enter}');
    expect(onSave).not.toHaveBeenCalled();
    expect(field).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByText('No .. or empty parts')).toBeInTheDocument();
  });
});
