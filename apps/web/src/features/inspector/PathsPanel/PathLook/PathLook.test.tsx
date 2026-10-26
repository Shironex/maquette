import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { ScenePath } from '@/ipc/client';
import { EMPTY_PATHS, type PathsDraft } from '@/state/paths';
import PathLook from './PathLook';

const PATH: ScenePath = {
  id: 'cord',
  name: 'Cord',
  curve: 'polyline',
  points: [
    [0, 0, 0],
    [1, 0, 0],
  ],
  start: null,
  end: null,
  width: 0.02,
  color: '#3a3936',
  glow: '#ffe08a',
};

describe('the look section', () => {
  it('renames the path through the draft', async () => {
    const change = vi.fn();
    render(<PathLook path={PATH} change={change} />);
    expect(screen.getByRole('textbox', { name: 'Width' })).toHaveValue('2.0');
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 's');
    const edit = change.mock.lastCall![0] as (d: PathsDraft, id: string) => PathsDraft;
    const next = edit({ ...EMPTY_PATHS, paths: [PATH] }, 'cord');
    expect(next.paths[0]!.name).toBe('Cords');
  });
});
