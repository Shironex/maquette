import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { usePoseKeysStore } from '@/state/use-pose-keys-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import PoseKeysBar from './PoseKeysBar';

beforeEach(() => usePoseKeysStore.setState(usePoseKeysStore.getInitialState(), true));
afterEach(clearMocks);

describe('the turn gizmo bar', () => {
  it('shows only while a pose key change is picked, with the turn and the shortcuts', async () => {
    await openRecipe('slash-with-a-lean');
    const { rerender } = render(<PoseKeysBar />);
    expect(screen.queryByRole('toolbar', { name: 'Turn gizmo' })).not.toBeInTheDocument();
    act(() => {
      useRecipeEditorStore.getState().pick({ what: 'layer', id: 'l-1' });
      usePoseKeysStore.getState().setFrame(20);
    });
    rerender(<PoseKeysBar />);
    expect(screen.getByRole('toolbar', { name: 'Turn gizmo' })).toBeVisible();
    expect(screen.getByText(/^Chest\s+X 12\.0°\s+Y 18\.0°\s+Z 0\.0°$/)).toBeVisible();
    expect(screen.getByText('Frame 20 has a key')).toBeVisible();
    expect(screen.getByText('mirror pose')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'World' }));
    expect(usePoseKeysStore.getState().space).toBe('world');
    await userEvent.click(screen.getByRole('button', { name: 'Hide the turn rings' }));
    expect(usePoseKeysStore.getState().turning).toBe(false);
  }, 15_000);
});
