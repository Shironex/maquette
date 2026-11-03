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
          layoutKey: 'k-seen',
        },
        busy: false,
        error: null,
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
    // The rebase carries the key of the layout.json this comparison was made against.
    expect(rebase).toHaveBeenCalledWith('moon-island', 'rebase', false, 'k-seen');
  });

  it('says when layout.json changed since the comparison and offers to review again', async () => {
    usePlansStore.setState({ project: 'moon-island' });
    vi.spyOn(usePlansStore.getState(), 'save').mockResolvedValue(true);
    const report = {
      taken: [],
      added: ['Lantern'],
      removed: [],
      clashes: [],
      change: null,
      layoutKey: 'k-old',
    };
    usePlanCameraStore.setState({
      project: 'moon-island',
      compare: { report, busy: false, error: null },
    });
    const stale =
      'layout.json changed since this comparison was shown, so nothing was rebased. Review again to see the changes as they are now.';
    const rebase = vi
      .spyOn(commands, 'planCameraRebase')
      .mockRejectedValueOnce({ code: 'invalid', message: stale })
      .mockResolvedValueOnce({
        view: usePlanCameraStore.getState().view as never,
        report: { ...report, layoutKey: 'k-new' },
      });
    render(<CameraCompare />);
    await userEvent.click(screen.getByRole('button', { name: 'Rebase' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Review again');
    expect(screen.queryByRole('button', { name: 'Rebase' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Review again' }));
    expect(rebase).toHaveBeenLastCalledWith('moon-island', 'rebase', true, null);
    expect(await screen.findByRole('button', { name: 'Rebase' })).toBeInTheDocument();
    expect(usePlanCameraStore.getState().compare?.report.layoutKey).toBe('k-new');
  });
});
