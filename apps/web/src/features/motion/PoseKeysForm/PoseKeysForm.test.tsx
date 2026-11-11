import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useGripSaveStore } from '@/state/use-grip-save-store';
import { usePoseKeysStore } from '@/state/use-pose-keys-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import PoseKeysForm from './PoseKeysForm';

const DEMO = 'slash-with-a-lean';

const keys = () => useRecipeEditorStore.getState().editing?.draft.layers[0]?.keys ?? [];
const keyOn = (clip: number, frame: number) =>
  keys().find(k => (k.clip ?? 0) === clip && k.frame === frame);

beforeEach(() => {
  usePoseKeysStore.setState(usePoseKeysStore.getInitialState(), true);
});
afterEach(clearMocks);

describe('the pose keys form', () => {
  it('shows the picked bone at the playhead, with its key, ease and limits', async () => {
    await openRecipe(DEMO);
    act(() => usePoseKeysStore.getState().setFrame(20));
    render(<PoseKeysForm id="l-1" />);
    expect(screen.getByRole('heading', { name: 'Pose keys, chest' })).toBeVisible();
    expect(screen.getByText('Chest has a key on this frame.')).toBeVisible();
    expect(screen.getByLabelText('Turn X')).toHaveValue('12');
    expect(screen.getByLabelText('Turn Y')).toHaveValue('18');
    expect(screen.getByRole('button', { name: 'Out', pressed: true })).toBeVisible();
    expect(screen.getByText('Inside the joint limits')).toBeVisible();
    expect(screen.getByText('Chest bend 12 of 40 degrees, twist 18 of 45.')).toBeVisible();
    // The body map fills the dots of keyed bones.
    expect(screen.getByRole('button', { name: 'Right upper arm, 2 keys' })).toBeVisible();
    // The first test opens the mock workspace cold.
  }, 15_000);

  it('keys a turn on the frame, warns past a limit, and eases', async () => {
    await openRecipe(DEMO);
    act(() => usePoseKeysStore.getState().setFrame(24));
    render(<PoseKeysForm id="l-1" />);
    await userEvent.clear(screen.getByLabelText('Turn Y'));
    await userEvent.type(screen.getByLabelText('Turn Y'), '60{Enter}');
    expect(keyOn(0, 24)?.pose.chest?.[1]).toBe(60);
    expect(screen.getByText('Past the joint limits')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Step' }));
    expect(keyOn(0, 24)?.ease).toBe('step');
    await userEvent.click(screen.getByRole('button', { name: 'Replace it' }));
    expect(useRecipeEditorStore.getState().editing?.draft.layers[0]?.mode).toBe('replace');
  });

  it('picks a bone from the map, mirrors a pose and removes the key', async () => {
    await openRecipe(DEMO);
    act(() => usePoseKeysStore.getState().setFrame(12));
    render(<PoseKeysForm id="l-1" />);
    await userEvent.click(screen.getByRole('button', { name: 'Right upper arm, 2 keys' }));
    expect(screen.getByRole('heading', { name: 'Pose keys, right upper arm' })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Mirror pose' }));
    expect(keyOn(0, 12)?.pose).toEqual({ leftUpperArm: [0, 10, -25] });
    await userEvent.click(screen.getByRole('button', { name: 'Remove key' }));
    expect(keyOn(0, 12)).toBeUndefined();
  });

  it('runs the shortcuts: K keys, M mirrors, R turns the gizmo off', async () => {
    await openRecipe(DEMO);
    act(() => usePoseKeysStore.getState().setFrame(26));
    render(<PoseKeysForm id="l-1" />);
    fireEvent.keyDown(window, { key: 'k' });
    expect(keyOn(0, 26)?.pose.chest).toBeDefined();
    fireEvent.keyDown(window, { key: 'r' });
    expect(usePoseKeysStore.getState().turning).toBe(false);
    fireEvent.keyDown(window, { key: 'm' });
    expect(keyOn(0, 26)?.pose.chest).toBeDefined();
  });
});

describe('save a grip from the pose keys', () => {
  it('waits for a key on a hand or a finger', async () => {
    await openRecipe(DEMO);
    render(<PoseKeysForm id="l-1" />);
    expect(screen.getByRole('button', { name: 'Save a grip' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('opens the save a grip sheet on the hands at the playhead', async () => {
    await openRecipe('scythe-charge-held');
    act(() => usePoseKeysStore.getState().setFrame(30));
    render(<PoseKeysForm id="l-1" />);
    await userEvent.click(screen.getByRole('button', { name: 'Save a grip' }));
    expect(useGripSaveStore.getState().sheet).toMatchObject({
      project: 'frostveil',
      recipe: 'scythe-charge-held',
      layer: 'l-1',
      frame: 30,
    });
    useGripSaveStore.setState(useGripSaveStore.getInitialState(), true);
  });
});
