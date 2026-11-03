import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, openPlans } from '@/test/open-plan';
import CanvasSwitches from './CanvasSwitches';

afterEach(clearMocks);

const AUTO = /^Auto layout\s*after agents$/;

const show = async (id: string) => {
  await openPlans('systems');
  await usePlansStore.getState().openCanvas(id);
  render(<CanvasSwitches />);
};

describe('CanvasSwitches', () => {
  it('turns Auto layout after agents off and on, each a change in the log', async () => {
    await show('combat-ideas');
    const auto = screen.getByRole('button', { name: AUTO });
    expect(auto).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByRole('button', { name: /^Overlaps/ })).not.toBeInTheDocument();
    await userEvent.click(auto);
    await screen.findByRole('button', { name: AUTO, pressed: false });
    expect(usePlansStore.getState().open?.plan.autoLayout).toBe(false);
    expect(usePlanLogStore.getState().lines[0]?.summary).toBe(
      'Turned auto layout after agents off'
    );
    await userEvent.click(screen.getByRole('button', { name: AUTO }));
    await screen.findByRole('button', { name: AUTO, pressed: true });
  });

  it('opens the Overlaps panel of a Game mechanics canvas, with the count', async () => {
    await show('combat-systems');
    const button = screen.getByRole('button', { name: /^Overlaps,\s*2\s*found$/ });
    usePlanLogStore.getState().setPanel(true);
    await userEvent.click(button);
    expect(usePlanExtrasStore.getState().overlaps).toBe(true);
    // One side panel at a time.
    expect(usePlanLogStore.getState().panel).toBe(false);
  });
});
