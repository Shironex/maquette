import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, openPlans } from '@/test/open-plan';
import PlanList from './PlanList';

afterEach(clearMocks);

const renderList = async (plans: string | null = 'demo') => {
  const store = await openPlans(plans);
  return render(<PlanList list={store.list ?? []} opening={null} openError={null} />);
};

describe('PlanList', () => {
  it('shows recent canvases first and finds one by name', async () => {
    await renderList();
    const recent = screen.getByRole('region', { name: 'Recent canvases' });
    const cards = within(recent).getAllByRole('button');
    expect(cards[0]).toHaveAccessibleName(/^Combat ideas 6 nodes/);
    await userEvent.type(screen.getByRole('textbox', { name: 'Find a canvas' }), 'story');
    const table = screen.getByRole('region', { name: 'All canvases' });
    expect(within(table).getAllByRole('row')).toHaveLength(2);
    expect(within(table).getByText('plans/opening-story.plan.json')).toBeInTheDocument();
    await userEvent.clear(screen.getByRole('textbox', { name: 'Find a canvas' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Find a canvas' }), 'nothing');
    expect(within(table).getByText('No canvas is called that.')).toBeInTheDocument();
  });

  it('makes a new canvas from the empty list', async () => {
    await renderList(null);
    expect(screen.getByText('No canvases yet')).toBeInTheDocument();
    await userEvent.click(screen.getAllByRole('button', { name: 'New canvas' })[0]!);
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 'Lantern festival');
    await userEvent.click(screen.getByRole('button', { name: 'Create' }));
    await vi.waitFor(() => expect(usePlansStore.getState().open?.id).toBe('lantern-festival'));
  });

  it('removes a canvas after asking', async () => {
    await renderList();
    await userEvent.click(screen.getByRole('button', { name: 'Remove Opening story' }));
    const dialog = screen.getByRole('dialog', { name: 'Remove Opening story?' });
    expect(dialog).toHaveTextContent('moves into the workspace');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Remove' }));
    await vi.waitFor(() =>
      expect(usePlansStore.getState().list?.map(p => p.id)).toEqual([
        'boss-arena',
        'boss-frost-warden',
        'combat-ideas',
      ])
    );
  });
});
