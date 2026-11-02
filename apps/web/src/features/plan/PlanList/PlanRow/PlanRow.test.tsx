import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { PlanSummary } from '@/ipc/client';
import PlanRow, { type PlanRowProps } from './PlanRow';

const plan: PlanSummary = {
  id: 'boss-arena',
  file: 'plans/boss-arena.plan.json',
  name: 'Boss arena',
  preset: 'brainstorm',
  nodes: 4,
  edges: 1,
  images: 0,
  links: 0,
  assets: 0,
  stops: 0,
  camera: false,
  edited: '2026-10-31T09:00:00.000Z',
  problem: null,
};

const renderRow = (props: Partial<PlanRowProps> = {}) => {
  const handlers = {
    onOpen: vi.fn(),
    onRename: vi.fn(),
    onType: vi.fn(),
    onCommit: vi.fn(),
    onCancel: vi.fn(),
    onRemove: vi.fn(),
  };
  render(
    <table>
      <tbody>
        <PlanRow plan={plan} edited="2 days ago" renaming={null} {...handlers} {...props} />
      </tbody>
    </table>
  );
  return handlers;
};

describe('PlanRow', () => {
  it('opens, renames and removes', async () => {
    const h = renderRow();
    await userEvent.click(screen.getByRole('button', { name: 'Boss arena' }));
    await userEvent.click(screen.getByRole('button', { name: 'Rename Boss arena' }));
    await userEvent.click(screen.getByRole('button', { name: 'Remove Boss arena' }));
    expect(h.onOpen).toHaveBeenCalledOnce();
    expect(h.onRename).toHaveBeenCalledOnce();
    expect(h.onRemove).toHaveBeenCalledOnce();
  });

  it('saves a rename on Enter and keeps the old name on Escape', async () => {
    const h = renderRow({ renaming: 'Warden arena' });
    const field = screen.getByRole('textbox', { name: 'New name for Boss arena' });
    expect(field).toHaveFocus();
    await userEvent.type(field, '!');
    expect(h.onType).toHaveBeenLastCalledWith('Warden arena!');
    await userEvent.keyboard('{Enter}');
    expect(h.onCommit).toHaveBeenCalled();
    await userEvent.keyboard('{Escape}');
    expect(h.onCancel).toHaveBeenCalledOnce();
  });

  it('shows why a broken file cannot open and offers only Remove', () => {
    renderRow({ plan: { ...plan, problem: 'The file is not a canvas' } });
    expect(screen.getByText('The file is not a canvas')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Rename Boss arena' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Remove Boss arena' })).toBeInTheDocument();
  });
});
