import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useRetargetStore } from '@/state/use-retarget-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import RigCheck from './RigCheck';
import { rigLines } from './RigCheck.hooks';

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

  it('fails weights past four influences', async () => {
    const s = await openMotion();
    const figure = s.view!.models.find(m => m.asset === 'clay-figure')!;
    const heavy = { ...figure, rig: { ...figure.rig!, maxInfluences: 6 } };
    expect(rigLines(heavy)[1]).toMatchObject({ status: 'fail' });
  });
});
