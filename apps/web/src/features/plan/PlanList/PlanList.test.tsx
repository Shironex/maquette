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
    expect(cards[0]).toHaveAccessibleName(/^Combat ideas Brainstorm 6 nodes/);
    // A long name may end in an ellipsis; hovering shows all of it.
    expect(within(cards[1]!).getByTitle('Boss: Frost Warden')).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: 'Find a canvas' }), 'story');
    const table = screen.getByRole('region', { name: 'All canvases' });
    expect(within(table).getAllByRole('row')).toHaveLength(2);
    expect(within(table).getByText('plans/opening-story.plan.json')).toBeInTheDocument();
    expect(within(table).getByRole('columnheader', { name: 'Preset' })).toBeInTheDocument();
    expect(within(table).getAllByRole('row')[1]).toHaveTextContent('Story');
    await userEvent.clear(screen.getByRole('textbox', { name: 'Find a canvas' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Find a canvas' }), 'nothing');
    expect(within(table).getByText('No canvas is called that.')).toBeInTheDocument();
  });

  it('makes a new canvas from the empty list', async () => {
    await renderList(null);
    expect(screen.getByText('Plan with canvases')).toBeInTheDocument();
    // Or an agent starts one from a prompt to copy.
    expect(screen.getByRole('region', { name: 'Or let an agent start one' })).toHaveTextContent(
      'using the Plan tools'
    );
    await userEvent.click(screen.getByRole('button', { name: 'New canvas' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 'Lantern festival');
    await userEvent.click(screen.getByRole('button', { name: 'Create' }));
    await vi.waitFor(() => expect(usePlansStore.getState().open?.id).toBe('lantern-festival'));
    expect(usePlansStore.getState().open?.plan.nodes).toEqual([]);
  });

  it('starts the empty list from the preset picked there', async () => {
    await renderList(null);
    const picker = screen.getByRole('radiogroup', { name: 'Start from' });
    expect(within(picker).getByRole('radio', { name: /^Brainstorm/ })).toBeChecked();
    await userEvent.click(within(picker).getByRole('radio', { name: /^Story/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Create Story canvas' }));
    const dialog = screen.getByRole('dialog', { name: 'New canvas' });
    expect(within(dialog).getByRole('radio', { name: /^Story/ })).toBeChecked();
    await userEvent.type(within(dialog).getByRole('textbox', { name: 'Name' }), 'Tour story');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Create' }));
    await vi.waitFor(() => expect(usePlansStore.getState().open?.id).toBe('tour-story'));
    const plan = usePlansStore.getState().open?.plan;
    expect(plan?.preset).toBe('story');
    expect(plan?.nodes.map(n => n.title)).toContain('Beat 1: First view');
  });

  it('never gives a new canvas the camera plan id', async () => {
    await renderList(null);
    await userEvent.click(screen.getByRole('button', { name: 'New canvas' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 'Camera plan');
    await userEvent.click(screen.getByRole('button', { name: 'Create' }));
    await vi.waitFor(() => expect(usePlansStore.getState().open?.id).toBe('camera-plan-2'));
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
