import { fireEvent, render, screen } from '@testing-library/react';
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
  });
});
