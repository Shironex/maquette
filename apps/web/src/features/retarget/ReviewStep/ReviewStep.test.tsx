import { fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useRetargetStore } from '@/state/use-retarget-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import ReviewStep from './ReviewStep';

afterEach(() => {
  clearMocks();
  useRetargetStore.setState({ sheet: null });
});

describe('the retarget review', () => {
  it('lists looks first, says why a clip failed and selects another', async () => {
    await openMockLibrary('demo');
    await useAnimationsStore.getState().load();
    await useRetargetStore.getState().open({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['sword-and-shield-idle', 'sword-and-shield-run', 'sword-and-shield-crouch'],
      step: 'settings',
    });
    await useRetargetStore.getState().retarget();
    render(<ReviewStep />);
    const list = within(screen.getByRole('complementary', { name: 'Baked clips' }));
    const buttons = list.getAllByRole('button');
    expect(buttons[0]).toHaveTextContent('sword_run');
    expect(buttons[1]).toHaveTextContent('sword_crouch');
    expect(screen.getAllByText(/left foot slides 4.2 cm/)[0]).toBeInTheDocument();
    expect(screen.getByText(/sword_crouch is left out until fixed/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /sword_idle/ }));
    expect(useRetargetStore.getState().sheet!.selected).toBe('sword-and-shield-idle');
  });

  it('marks the flagged frames on the strip and takes a clip as it is', async () => {
    await openMockLibrary('demo');
    await useAnimationsStore.getState().load();
    await useRetargetStore.getState().open({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['sword-and-shield-idle', 'sword-and-shield-run'],
      step: 'settings',
    });
    await useRetargetStore.getState().retarget();
    render(<ReviewStep />);
    expect(screen.getByText('Slides 4.2 cm, frames 9 to 14')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Accept as is' }));
    expect(useRetargetStore.getState().sheet!.accepted).toEqual(['sword-and-shield-run']);
    expect(screen.getByText(/2 good · 0 to look/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Accept as is' })).toBeNull();
  });
});
