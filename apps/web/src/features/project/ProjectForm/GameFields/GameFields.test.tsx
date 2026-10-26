import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { inspectGameRepo } from '@/ipc/mock/game-fixtures';
import { emptyProjectFields } from '@/state/onboarding';
import { GameFields } from '.';

describe('GameFields', () => {
  it('marks the detected format and changes the format and folder', async () => {
    const onChange = vi.fn();
    const value = {
      ...emptyProjectFields('Maze game'),
      target: 'game' as const,
      game: inspectGameRepo('/projects/maze-game'),
      gameFormat: 'obj' as const,
    };
    render(<GameFields value={value} size="md" onChange={onChange} />);
    const obj = screen.getByRole('button', { name: 'Wavefront .obj + .mtl (detected)' });
    expect(obj).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'glTF binary .glb' }));
    expect(onChange).toHaveBeenCalledWith({ gameFormat: 'glb' });
    await userEvent.type(screen.getByRole('textbox', { name: 'Model folder' }), 'x');
    expect(onChange).toHaveBeenLastCalledWith({ modelsDir: 'assets/modelsx' });
    expect(screen.getByText(/skip Capture posters and camera stops/)).toBeInTheDocument();
  });
});
