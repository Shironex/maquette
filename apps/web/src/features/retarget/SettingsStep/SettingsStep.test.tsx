import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useRetargetStore } from '@/state/use-retarget-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import SettingsStep from './SettingsStep';

afterEach(() => {
  clearMocks();
  useRetargetStore.setState({ sheet: null });
});

describe('the clip settings step', () => {
  it('names clips in snake_case and edits the selected one', async () => {
    await openMockLibrary('demo');
    await useAnimationsStore.getState().load();
    await useRetargetStore.getState().open({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['standing-dodge-left', 'sword-and-shield-slash'],
      step: 'settings',
    });
    render(<SettingsStep />);
    expect(screen.getAllByText('dodge_left')[0]).toBeInTheDocument();
    expect(screen.getByText('sword_slash')).toBeInTheDocument();
    expect(screen.getByText('update')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'In place' }));
    expect(useRetargetStore.getState().sheet!.settings[0]!.rootMotion).toBe(false);
    fireEvent.change(screen.getByLabelText('Name in the model'), { target: { value: 'dodge_l' } });
    expect(useRetargetStore.getState().sheet!.settings[0]!.name).toBe('dodge_l');
    fireEvent.click(screen.getByRole('button', { name: 'Reset names' }));
    expect(useRetargetStore.getState().sheet!.settings[0]!.name).toBe('dodge_left');
    fireEvent.click(screen.getByRole('switch', { name: /Pin the feet/ }));
    expect(useRetargetStore.getState().sheet!.settings[0]!.pinFeet).toBe(true);
    // The trim handles move a frame at a time from the keyboard, and never cross.
    const trimIn = screen.getByRole('slider', { name: 'Trim in' });
    fireEvent.keyDown(trimIn, { key: 'ArrowRight' });
    expect(useRetargetStore.getState().sheet!.settings[0]!.start).toBe(1);
    expect(screen.getByText(/^in 1 · out /)).toBeInTheDocument();
    act(() => {
      useRetargetStore.getState().changeSetting('standing-dodge-left', { start: 5, end: 6 });
    });
    fireEvent.keyDown(screen.getByRole('slider', { name: 'Trim out' }), { key: 'ArrowLeft' });
    expect(useRetargetStore.getState().sheet!.settings[0]!.end).toBe(6);
  });
});
