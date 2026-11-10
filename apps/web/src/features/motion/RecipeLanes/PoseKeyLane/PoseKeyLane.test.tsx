import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { usePoseKeysStore } from '@/state/use-pose-keys-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import PoseKeyLane from './PoseKeyLane';
import { draggedTo } from './PoseKeyLane.hooks';

const keys = () => useRecipeEditorStore.getState().editing?.draft.layers[0]?.keys ?? [];
const frames = () => keys().map(k => `${k.clip ?? 0}:${k.frame}:${Object.keys(k.pose).join('+')}`);

beforeEach(() => usePoseKeysStore.setState(usePoseKeysStore.getInitialState(), true));
afterEach(clearMocks);

describe('a pose key lane', () => {
  it('turns a drag across the lane into a frame', () => {
    expect(draggedTo(20, 50, 500, 100)).toBe(30);
    expect(draggedTo(20, -500, 500, 100)).toBe(0);
    expect(draggedTo(20, 10, 0, 100)).toBe(20);
  });

  it("puts a diamond on each of a bone's keys, moves one with the arrows and copies with Alt", async () => {
    await openRecipe('slash-with-a-lean');
    render(<PoseKeyLane id="l-1" bone="chest" frames={100} />);
    const diamonds = screen.getAllByRole('button', { name: /^Chest key at frame/ });
    expect(diamonds).toHaveLength(3);
    act(() => diamonds[1]!.focus());
    fireEvent.keyDown(diamonds[1]!, { key: 'ArrowRight' });
    expect(frames()).toContain('0:21:chest');
    expect(frames()).not.toContain('0:20:chest');
    const moved = screen.getByRole('button', { name: /^Chest key at frame 21/ });
    fireEvent.keyDown(moved, { key: 'ArrowRight', shiftKey: true, altKey: true });
    expect(frames()).toEqual(expect.arrayContaining(['0:21:chest', '0:31:chest']));
  }, 15_000);

  it('goes to the key on Enter', async () => {
    await openRecipe('slash-with-a-lean');
    render(<PoseKeyLane id="l-1" bone={null} frames={100} />);
    const all = screen.getAllByRole('button', { name: /^Pose key at frame/ });
    expect(all.length).toBeGreaterThanOrEqual(5);
    fireEvent.keyDown(all[2]!, { key: 'Enter' });
    expect(usePoseKeysStore.getState().frame).toBe(20);
  });
});
