import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { InboxItem } from '@/ipc/client';
import { useInboxStore } from '@/state/use-inbox-store';
import { useTriageStore } from '@/state/use-triage-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { TriageTable } from '.';

let items: InboxItem[] = [];

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'triage');
  await useInboxStore.getState().load();
  items = useInboxStore.getState().snapshot!.items;
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null, checked: [] });
  useTriageStore.setState({ layout: 'one' });
});

describe('the inbox triage table', () => {
  it('groups the models by what they need, worst last, with where each goes', () => {
    render(<TriageTable items={items} checked={[]} onCheck={() => {}} onCheckAll={() => {}} />);
    const groups = screen.getAllByRole('rowgroup').slice(1);
    expect(groups.map(g => g.getAttribute('aria-label'))).toEqual([
      'Checks pass',
      'To review',
      'Replaced by a redo',
      'Unreadable',
    ]);
    const lantern = within(groups[0]!).getByRole('row', { name: /Lantern post 3d model/ });
    expect(lantern).toHaveTextContent('Crystal lantern');
    expect(lantern).toHaveTextContent('Frost cathedral');
    expect(lantern).toHaveTextContent('v2');
    expect(lantern).toHaveTextContent('attached');
    expect(within(groups[2]!).getAllByText(/accepted$/)).toHaveLength(2);
  });

  it('ticks one model or a whole group', async () => {
    const onCheck = vi.fn();
    const onCheckAll = vi.fn();
    render(<TriageTable items={items} checked={[]} onCheck={onCheck} onCheckAll={onCheckAll} />);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Frozen angel' }));
    expect(onCheck).toHaveBeenCalledWith('frozen_angel.glb', true);
    await userEvent.click(
      screen.getByRole('checkbox', { name: 'Select every model in Replaced by a redo' })
    );
    expect(onCheckAll).toHaveBeenCalledWith(
      ['crystal_lantern_redo.glb', 'observatory_arch_v3.glb'],
      true
    );
  });

  it('opens a model one by one from its name', async () => {
    render(<TriageTable items={items} checked={[]} onCheck={() => {}} onCheckAll={() => {}} />);
    await userEvent.click(screen.getByRole('button', { name: 'Frozen angel' }));
    expect(useInboxStore.getState().selected).toBe('frozen_angel.glb');
    expect(useTriageStore.getState().layout).toBe('one');
  });

  it('folds a long group behind N more', async () => {
    const gate = items.find(i => i.id === 'veil_gate.glb')!;
    const many = Array.from({ length: 12 }, (_, i) => ({ ...gate, id: `copy-${i}.glb` }));
    render(<TriageTable items={many} checked={[]} onCheck={() => {}} onCheckAll={() => {}} />);
    expect(screen.getAllByRole('checkbox')).toHaveLength(9);
    await userEvent.click(screen.getByRole('button', { name: '4 more in checks pass' }));
    expect(screen.getAllByRole('checkbox')).toHaveLength(13);
  });
});
