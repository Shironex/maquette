import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, openPlans } from '@/test/open-plan';
import NewCanvasDialog from './NewCanvasDialog';

afterEach(clearMocks);

describe('NewCanvasDialog', () => {
  it('asks for a name, then makes and opens the canvas on Enter', async () => {
    await openPlans();
    const onClose = vi.fn();
    render(<NewCanvasDialog onClose={onClose} />);
    const name = screen.getByRole('textbox', { name: 'Name' });
    expect(name).toHaveFocus();
    expect(screen.getByRole('button', { name: 'Create' })).toHaveAttribute('aria-disabled', 'true');
    await userEvent.type(name, 'Warden fight{Enter}');
    await vi.waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(usePlansStore.getState().open?.plan.name).toBe('Warden fight');
  });

  it('counts a name made only of white space the core trims as blank', async () => {
    await openPlans();
    const onClose = vi.fn();
    render(<NewCanvasDialog onClose={onClose} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), '\u0085\uFEFF{Enter}');
    expect(screen.getByRole('button', { name: 'Create' })).toHaveAttribute('aria-disabled', 'true');
    expect(onClose).not.toHaveBeenCalled();
    expect(usePlansStore.getState().list).toHaveLength(4);
  });

  it('closes on Cancel without making anything', async () => {
    await openPlans();
    const onClose = vi.fn();
    render(<NewCanvasDialog onClose={onClose} />);
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onClose).toHaveBeenCalledOnce();
    expect(usePlansStore.getState().list).toHaveLength(4);
  });
});
