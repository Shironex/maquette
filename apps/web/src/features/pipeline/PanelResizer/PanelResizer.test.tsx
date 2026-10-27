import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { PanelResizer } from '.';

describe('the side panel divider', () => {
  it('widens and narrows the panel with the keys and puts it back on a double click', () => {
    const onChange = vi.fn();
    const onReset = vi.fn();
    render(
      <div data-resize-row>
        <PanelResizer
          width={540}
          onChange={onChange}
          onReset={onReset}
          label="Compression presets"
        />
      </div>
    );
    const handle = screen.getByRole('separator', { name: 'Resize the compression presets' });
    expect(handle).toHaveAttribute('aria-valuenow', '540');
    fireEvent.keyDown(handle, { key: 'ArrowLeft' });
    expect(onChange).toHaveBeenLastCalledWith(564);
    fireEvent.keyDown(handle, { key: 'ArrowRight' });
    expect(onChange).toHaveBeenLastCalledWith(516);
    fireEvent.keyDown(handle, { key: 'End' });
    expect(onChange).toHaveBeenLastCalledWith(380);
    fireEvent.keyDown(handle, { key: 'a' });
    expect(onChange).toHaveBeenCalledTimes(3);
    fireEvent.doubleClick(handle);
    expect(onReset).toHaveBeenCalledOnce();
  });
});
