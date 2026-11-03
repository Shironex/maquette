import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import CanvasEmpty from './CanvasEmpty';

describe('CanvasEmpty', () => {
  it('offers a button for every way to start, and a prompt naming the canvas', async () => {
    const onAdd = vi.fn();
    render(
      <CanvasEmpty
        file="plans/boss-arena.plan.json"
        name="Boss arena"
        camera={false}
        onAdd={onAdd}
        keys={null}
      />
    );
    const empty = screen.getByRole('region', { name: 'This canvas is empty' });
    expect(empty).toHaveTextContent('saved as plans/boss-arena.plan.json');
    for (const [name, kind] of [
      ['Write a note', 'note'],
      ['Picture', 'image'],
      ['Web page', 'link'],
      ['Pick assets', 'asset'],
    ] as const) {
      await userEvent.click(screen.getByRole('button', { name }));
      expect(onAdd).toHaveBeenLastCalledWith(kind);
    }
    expect(screen.getByRole('region', { name: 'Or ask an agent to start it' })).toHaveTextContent(
      '"Boss arena" (plans/boss-arena.plan.json)'
    );
  });

  it('points the empty camera plan at Add stop', () => {
    render(
      <CanvasEmpty
        file="plans/camera-plan.plan.json"
        name="Camera plan"
        camera
        onAdd={vi.fn()}
        keys={null}
      />
    );
    expect(screen.getByRole('region', { name: 'No camera stops yet' })).toHaveTextContent(
      'Add stop in the camera panel'
    );
    expect(screen.queryByRole('button')).toBeNull();
  });
});
