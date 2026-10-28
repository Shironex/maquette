import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAnimationsStore } from '@/state/use-animations-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import SaveClipsSheet from './SaveClipsSheet';

afterEach(() => {
  clearMocks();
  useAnimationsStore.setState({ view: null, saving: null });
});

describe('the save clips sheet', () => {
  it('lists the model clips, chosen by default, and saves the ones left on', async () => {
    await openMockLibrary('demo');
    await useAnimationsStore.getState().load();
    await useAnimationsStore
      .getState()
      .openSave({ project: null, asset: null, inbox: 'elsheria.glb' });
    render(<SaveClipsSheet />);
    const sheet = screen.getByRole('dialog', { name: 'Save clips to the library' });
    expect(sheet).toHaveTextContent('Mixamo rig, 119 bones');
    const boxes = within(sheet).getAllByRole('checkbox');
    expect(boxes.every(b => (b as HTMLInputElement).checked)).toBe(true);
    expect(within(sheet).getByLabelText('Pack')).toHaveValue('elsheria moves');
    expect(sheet).toHaveTextContent('New pack');
    await userEvent.click(boxes[0]!);
    const n = boxes.length - 1;
    await userEvent.click(
      within(sheet).getByRole('button', { name: n === 1 ? 'Save 1 clip' : `Save ${n} clips` })
    );
    expect(useAnimationsStore.getState().saving).toBeNull();
    const saved = useAnimationsStore
      .getState()
      .view!.items.filter(i => i.clip.pack === 'elsheria moves');
    expect(saved).toHaveLength(n);
  });
});
