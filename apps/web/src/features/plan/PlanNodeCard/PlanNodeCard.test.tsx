import { fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ReactFlowProvider } from '@xyflow/react';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, fakeFlowLayout, openPlans } from '@/test/open-plan';
import { PlanCanvas } from '../PlanCanvas';

beforeAll(fakeFlowLayout);
afterEach(clearMocks);

/** The cards draw inside the canvas, which gives them React Flow's node context. */
const renderNodes = async () => {
  await openPlans();
  await usePlansStore.getState().openCanvas('combat-ideas');
  render(
    <ReactFlowProvider>
      <PlanCanvas />
    </ReactFlowProvider>
  );
};

const node = (label: string) => screen.getByRole('group', { name: label });
const stored = (id: string) => usePlansStore.getState().nodes.find(n => n.id === id)?.data.node;

describe('PlanNodeCard', () => {
  it('writes a note on double click and keeps it on Mod+Enter', async () => {
    await renderNodes();
    fireEvent.doubleClick(within(node('Note: Frost veil')).getByText(/Slow time/));
    const field = screen.getByRole('textbox', { name: 'Note text' });
    await userEvent.clear(field);
    await userEvent.type(field, 'Veil slows time{Control>}{Enter}{/Control}');
    expect(stored('n-01')?.text).toBe('Veil slows time');
    expect(screen.queryByRole('textbox', { name: 'Note text' })).toBeNull();
  });

  it('drops an edit on Escape and edits a caption with Enter', async () => {
    await renderNodes();
    fireEvent.doubleClick(within(node('Note: Frost veil')).getByText(/Slow time/));
    await userEvent.type(screen.getByRole('textbox', { name: 'Note text' }), ' more{Escape}');
    expect(stored('n-01')?.text).not.toContain('more');

    fireEvent.doubleClick(
      within(node('Picture: Veil gate at night')).getByText('Veil gate at night')
    );
    const caption = screen.getByRole('textbox', { name: 'Caption' });
    await userEvent.clear(caption);
    await userEvent.type(caption, 'Gate kit B{Enter}');
    expect(screen.getByRole('group', { name: 'Picture: Gate kit B' })).toBeInTheDocument();
  });

  it('opens and copies a link', async () => {
    await renderNodes();
    const open = vi.spyOn(commands, 'planOpenLink');
    const link = node('Link: Lighting small scenes at night');
    expect(within(link).getByText('example.com')).toBeInTheDocument();
    await userEvent.click(within(link).getByRole('button', { name: 'Open in the browser' }));
    expect(open).toHaveBeenCalledOnce();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } });
    await userEvent.click(within(link).getByRole('button', { name: 'Copy the address' }));
    expect(writeText).toHaveBeenCalledWith('https://example.com/lighting-small-scenes');
  });
});
