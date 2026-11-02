import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { usePlanCameraStore } from '@/state/use-plan-camera-store';
import { usePlansStore } from '@/state/use-plans-store';
import CameraCompare from './CameraCompare';

afterEach(() => {
  vi.restoreAllMocks();
  usePlanCameraStore.getState().clear();
});

describe('CameraCompare', () => {
  it('shows both sides and rebases or starts over only when asked', async () => {
    usePlansStore.setState({ project: 'moon-island' });
    vi.spyOn(usePlansStore.getState(), 'save').mockResolvedValue(true);
    usePlanCameraStore.setState({
      project: 'moon-island',
      compare: {
        report: {
          taken: ['Keystone: fov'],
          added: ['Lantern'],
          removed: [],
          clashes: ['Moon: fov changed in both; the plan’s value stays'],
          change: null,
        },
        busy: false,
      },
    });
    const rebase = vi
      .spyOn(commands, 'planCameraRebase')
      .mockRejectedValue({ code: 'io', message: 'x' });
    render(<CameraCompare />);
    expect(screen.getByRole('region', { name: 'A rebase takes in' })).toHaveTextContent(
      'Keystone: fov'
    );
    expect(screen.getByRole('region', { name: 'Stops it adds to the plan' })).toHaveTextContent(
      'Lantern'
    );
    expect(
      screen.getByRole('region', { name: 'Kept from the plan, changed on both sides' })
    ).toHaveTextContent('Moon: fov');
    expect(rebase).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Rebase' }));
    expect(rebase).toHaveBeenCalledWith('moon-island', 'rebase', false);
  });
});
