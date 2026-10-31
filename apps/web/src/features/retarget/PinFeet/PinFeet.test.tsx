import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useAppStore } from '@/state/use-app-store';
import { useRetargetStore } from '@/state/use-retarget-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { ReviewStep } from '../ReviewStep';

afterEach(() => {
  clearMocks();
  useRetargetStore.setState({ sheet: null });
  useAppStore.setState({ notice: null });
});

describe('pin the feet', () => {
  it('bakes a sliding clip again with its feet pinned and shows the slide before and after', async () => {
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
    fireEvent.click(screen.getByRole('button', { name: /sword_run/ }));
    expect(screen.getAllByText(/left foot slides 4.2 cm/).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('button', { name: 'Pin the feet and bake again' }));
    const pinned = await screen.findByRole('group', { name: 'Feet pinned' });
    expect(within(pinned).getByText('4.2 cm')).toBeInTheDocument();
    expect(within(pinned).getByText('0.3 cm')).toBeInTheDocument();
    const sheet = useRetargetStore.getState().sheet!;
    expect(sheet.selected).toBe('sword-and-shield-run');
    expect(sheet.settings.find(s => s.clip === 'sword-and-shield-run')?.pinFeet).toBe(true);
    expect(sheet.settings.find(s => s.clip === 'sword-and-shield-idle')?.pinFeet).toBeFalsy();
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: 'Pin the feet and bake again' })).toBeNull()
    );
  });

  it('leaves a pin off that slid more, says so and does not offer it again', async () => {
    await openMockLibrary('demo');
    await useAnimationsStore.getState().load();
    await useRetargetStore.getState().open({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['sword-and-shield-block'],
      step: 'settings',
    });
    await useRetargetStore.getState().retarget();
    render(<ReviewStep />);
    fireEvent.click(screen.getByRole('button', { name: 'Pin the feet and bake again' }));
    const off = await screen.findByRole('group', { name: 'Pin left off' });
    expect(within(off).getByText('8.9 cm')).toBeInTheDocument();
    expect(within(off).getByText('2.8 cm')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Pin the feet and bake again' })).toBeNull();
    const sheet = useRetargetStore.getState().sheet!;
    expect(sheet.settings[0]?.pinFeet).toBe(false);
  });

  it('opens the draft in Blender only from the button, and says when it cannot', async () => {
    await openMockLibrary('demo');
    await useAnimationsStore.getState().load();
    await useRetargetStore.getState().open({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['sword-and-shield-idle'],
      step: 'settings',
    });
    await useRetargetStore.getState().retarget();
    render(<ReviewStep />);
    expect(useAppStore.getState().notice).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Open in Blender' }));
    await waitFor(() =>
      expect(useAppStore.getState().notice?.title).toBe('Could not open Blender')
    );
  });
});
