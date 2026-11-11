import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { RecipeSheetFrame } from '@/ipc/client';
import RecipeSheet from './RecipeSheet';

const stills = vi.hoisted(() => vi.fn());
vi.mock('@/features/viewer/pose-snapshots', () => ({ poseStills: stills }));

const FRAMES: RecipeSheetFrame[] = [
  { frame: 12, kind: 'key', label: 'spine key', detail: null, warn: false },
  { frame: 36, kind: 'seam', label: 'seam 1', detail: 'hips jump 4.3 cm', warn: true },
];

describe('the pose sheet', () => {
  it('draws a still per picked frame, marks what warns and moves the player', async () => {
    stills.mockResolvedValue(['data:image/jpeg;base64,a', 'data:image/jpeg;base64,b']);
    const onPick = vi.fn();
    render(
      <RecipeSheet
        url="asset://bake.glb"
        clip="Bake"
        last={60}
        frames={FRAMES}
        current={36}
        onPick={onPick}
      />
    );
    expect(screen.getByText('Pose sheet, at seams, keys and holds')).toBeVisible();
    expect(stills).toHaveBeenCalledWith('asset://bake.glb', 'Bake', [12, 36], 60);
    const sheet = screen.getByRole('region', { name: 'Pose sheet' });
    const seam = within(sheet).getByRole('button', { name: /Frame 36, seam 1/ });
    expect(seam).toHaveAttribute('aria-pressed', 'true');
    expect(seam).toHaveTextContent('hips jump 4.3 cm');
    expect(within(seam).getByRole('img', { name: 'Worth a look' })).toBeInTheDocument();
    await waitFor(() => expect(sheet.querySelectorAll('img[src^="data:"]')).toHaveLength(2));
    await userEvent.click(within(sheet).getByRole('button', { name: /Frame 12, spine key/ }));
    expect(onPick).toHaveBeenCalledWith(12);
  });

  it('names the frame when the stills cannot be drawn, and hides with no frames', async () => {
    stills.mockRejectedValue(new Error('no WebGL'));
    const { rerender, container } = render(
      <RecipeSheet
        url="asset://bake.glb"
        clip="Bake"
        last={60}
        frames={FRAMES}
        current={-1}
        onPick={() => {}}
      />
    );
    expect(await screen.findByText('frame 36')).toBeVisible();
    rerender(
      <RecipeSheet url={null} clip="Bake" last={60} frames={[]} current={-1} onPick={() => {}} />
    );
    expect(container).toBeEmptyDOMElement();
  });
});
