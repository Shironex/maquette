import { act, render, renderHook, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { StageLoader, ViewLoader, useStageReady } from '.';

describe('the loaders', () => {
  it('marks the content area busy and says what is coming', () => {
    const { container } = render(<ViewLoader label="Opening Motion..." />);
    expect(container.querySelector('main[aria-busy="true"]')).not.toBeNull();
    expect(screen.getByRole('status')).toHaveTextContent('Opening Motion...');
  });

  it('lets clicks through to the stage under it', () => {
    const { container } = render(<StageLoader />);
    expect(screen.getByRole('status')).toHaveTextContent('Loading the model...');
    expect(container.firstChild).toHaveClass('pointer-events-none');
  });

  it('waits again when the stage moves to another model', () => {
    const { result, rerender } = renderHook(({ url }) => useStageReady(url), {
      initialProps: { url: 'a.glb' },
    });
    expect(result.current.ready).toBe(false);
    act(() => result.current.onLoaded());
    expect(result.current.ready).toBe(true);
    rerender({ url: 'b.glb' });
    expect(result.current.ready).toBe(false);
  });
});
