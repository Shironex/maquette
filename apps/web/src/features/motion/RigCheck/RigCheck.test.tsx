import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';
import { useRetargetStore } from '@/state/use-retarget-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import RigCheck from './RigCheck';
import { rigLines, vrmLine } from './RigCheck.hooks';

afterEach(() => clearMocks());

describe('the rig check', () => {
  it('names the missing bones and opens the bone map', async () => {
    await openMotion();
    render(<RigCheck />);
    expect(screen.getByText('2 bones the library expects are missing')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Map bones' }));
    await waitFor(() => expect(useRetargetStore.getState().sheet?.step).toBe('map'));
    useRetargetStore.setState({ sheet: null });
  });

  it('says a model without a rig still plays whole model clips', async () => {
    const s = await openMotion();
    const turntable = s.view!.models.find(m => m.asset === 'turntable')!;
    expect(rigLines(turntable)).toEqual([expect.objectContaining({ title: 'No rig' })]);
  });

  it('ends with the VRM readiness of a rig that has one', async () => {
    const s = await openMotion();
    const figure = s.view!.models.find(m => m.asset === 'clay-figure')!;
    expect(rigLines(figure).at(-1)).toMatchObject({
      id: 'vrm',
      status: 'pass',
      title: 'VRM ready, 20 of 55 bones',
    });
    const posed = vrmLine({
      ...figure.rig!.vrm!,
      ready: false,
      findings: [
        { id: 'facing', level: 'blocker', title: 'Faces +X, VRM wants +Z', detail: null },
        { id: 'ground', level: 'warning', title: 'Soles sit 3 cm above the ground', detail: null },
      ],
    });
    expect(posed).toMatchObject({
      status: 'warning',
      title: 'Not VRM ready: 1 thing to fix',
      detail: 'Faces +X, VRM wants +Z. Soles sit 3 cm above the ground.',
    });
  });

  it("opens a character's VRM export in the Avatar view", async () => {
    await openMotion('demo', 'clay-figure');
    render(<RigCheck />);
    fireEvent.click(screen.getByRole('button', { name: 'Open Avatar' }));
    expect(useAppStore.getState().view).toBe('avatar');
    expect(useAvatarStore.getState().asset).toBe('clay-figure');
    await waitFor(() => expect(useAvatarStore.getState().view?.name).toBe('Clay figure'));
  });

  it('fails weights past four influences', async () => {
    const s = await openMotion();
    const figure = s.view!.models.find(m => m.asset === 'clay-figure')!;
    const heavy = { ...figure, rig: { ...figure.rig!, maxInfluences: 6 } };
    expect(rigLines(heavy)[1]).toMatchObject({ status: 'fail' });
  });
});
