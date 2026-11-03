import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, openPlans } from '@/test/open-plan';
import OverlapsPanel from './OverlapsPanel';

afterEach(clearMocks);

const open = async () => {
  await openPlans('systems');
  await usePlansStore.getState().openCanvas('combat-systems');
  usePlanExtrasStore.getState().setOverlaps(true);
  render(<OverlapsPanel />);
  return screen.getByRole('complementary', { name: 'Overlaps' });
};

describe('OverlapsPanel', () => {
  it('words each overlap, shows the pair and marks one as fine', async () => {
    const panel = await open();
    expect(within(panel).getByText('2 found')).toBeInTheDocument();
    const look = within(panel).getByRole('group', { name: 'Sigils and Runes look the same' });
    expect(look).toHaveTextContent('Both looks come down to ring');
    expect(
      within(panel).getByRole('group', { name: 'Dash and Frost veil share Shift' })
    ).toBeInTheDocument();
    expect(panel).toHaveTextContent('Same main word counts as the same.');

    await userEvent.click(within(look).getByRole('button', { name: /^Show/ }));
    const selected = usePlansStore
      .getState()
      .nodes.filter(n => n.selected)
      .map(n => n.id);
    expect(selected).toEqual(['n-02', 'n-03']);

    await userEvent.click(within(look).getByRole('button', { name: /^Mark as fine/ }));
    expect(await within(panel).findByText('1 found')).toBeInTheDocument();
    const fine = within(panel).getByRole('region', { name: 'Marked as fine' });
    expect(
      within(fine).getByRole('group', { name: 'Sigils and Runes look the same' })
    ).toBeInTheDocument();
    expect(usePlansStore.getState().open?.plan.fine).toHaveLength(1);
  });

  it('closes', async () => {
    const panel = await open();
    await userEvent.click(within(panel).getByRole('button', { name: 'Close Overlaps' }));
    expect(usePlanExtrasStore.getState().overlaps).toBe(false);
  });
});
