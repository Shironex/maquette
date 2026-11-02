import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, openPlans } from '@/test/open-plan';
import RemoveCanvasDialog from './RemoveCanvasDialog';

afterEach(clearMocks);

describe('RemoveCanvasDialog', () => {
  it('says where the file goes, focuses Cancel and removes on confirm', async () => {
    const store = await openPlans();
    const plan = store.list!.find(p => p.id === 'boss-arena')!;
    const onClose = vi.fn();
    render(<RemoveCanvasDialog plan={plan} onClose={onClose} />);
    const dialog = screen.getByRole('dialog', { name: 'Remove Boss arena?' });
    expect(dialog).toHaveTextContent('plans/boss-arena.plan.json leaves the project');
    expect(dialog).toHaveTextContent('plans/images stay');
    await userEvent.click(screen.getByRole('button', { name: 'Remove' }));
    await vi.waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(usePlansStore.getState().list?.map(p => p.id)).not.toContain('boss-arena');
  });
});
