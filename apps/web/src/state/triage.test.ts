import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { InboxItem, ProviderJob } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import {
  acceptPlan,
  checksCell,
  filterItems,
  fromCell,
  groupCounts,
  groupItems,
  matchedCell,
  selectionNote,
  triageGroup,
  versionCell,
} from './triage';
import { useInboxStore } from './use-inbox-store';

let items: InboxItem[] = [];
const item = (id: string) => items.find(i => i.id === id)!;

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'triage');
  await useInboxStore.getState().load();
  items = useInboxStore.getState().snapshot!.items;
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null, checked: [] });
});

describe('inbox triage groups', () => {
  it('puts each model where its need is, a replaced one whatever its checks say', () => {
    expect(triageGroup(item('veil_gate.glb'))).toBe('pass');
    expect(triageGroup(item('elsheria.glb'))).toBe('review');
    expect(triageGroup(item('rune_obelisk.glb'))).toBe('unreadable');
    expect(triageGroup(item('crystal_lantern_redo.glb'))).toBe('replaced');
    expect(groupCounts(items)).toEqual({ all: 12, pass: 6, review: 3, replaced: 2, unreadable: 1 });
    expect(groupItems(items).map(g => g.label)).toEqual([
      'Checks pass',
      'To review',
      'Replaced by a redo',
      'Unreadable',
    ]);
    expect(filterItems(items, 'replaced').map(i => i.id)).toEqual([
      'crystal_lantern_redo.glb',
      'observatory_arch_v3.glb',
    ]);
    expect(filterItems(items, 'all')).toHaveLength(12);
  });

  it('fills the table cells from where each model goes and came from', () => {
    expect(versionCell(item('observatory_arch_v4.glb'))).toBe('v4');
    expect(versionCell(item('veil_platform_scan.glb'))).toBe('new');
    expect(checksCell(item('crystal_lantern_redo.glb'))).toBe('v2 accepted');
    expect(checksCell(item('elsheria.glb'))).toBe('Hair vertices weighted to finger bones');
    expect(checksCell(item('rune_obelisk.glb'))).toBe('could not read it');
    expect(fromCell(item('guardian_statues.glb'))).toBe('By hand');
    expect(fromCell(item('lantern_post_3d_model.glb'))).toBe('Download');
    expect(matchedCell(item('guardian_statues.glb'))).toBe('name');
    expect(matchedCell(item('frozen_angel.glb'))).toBe('API send');
    expect(matchedCell(item('veil_platform_scan.glb'))).toBe('none');
  });
});

describe('the accept plan', () => {
  it('counts new models, later versions, closing sends and skipped files', () => {
    const waiting = { provider: 'by-hand', state: 'waiting', project: 'frostveil' };
    const jobs = [
      { ...waiting, asset: 'guardian-statues' },
      { ...waiting, asset: 'elsewhere' },
    ] as ProviderJob[];
    const plan = acceptPlan(
      items,
      ['guardian_statues.glb', 'observatory_arch_v4.glb', 'rune_obelisk.glb'],
      jobs
    );
    expect(plan.ids).toEqual(['guardian_statues.glb', 'observatory_arch_v4.glb']);
    expect(plan.fresh).toBe(1);
    expect(plan.later).toEqual(['Observatory arch v4']);
    expect(plan.handoffs).toBe(1);
    expect(plan.skipped).toBe(1);
  });

  it('says what the selection holds', () => {
    expect(selectionNote(items, ['veil_gate.glb', 'frozen_angel.glb'])).toBe('all checks pass');
    expect(selectionNote(items, ['veil_gate.glb', 'elsheria.glb', 'rune_obelisk.glb'])).toBe(
      '1 to review, 1 unreadable'
    );
    expect(selectionNote(items, [])).toBe('');
  });
});
