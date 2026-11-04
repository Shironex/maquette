import { renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { Approval, McpStatus, RigRecord } from '@/ipc/client';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useRigStore } from '@/state/use-rig-store';
import { useFloatingApprovals } from './ApprovalPrompt.hooks';

const ask = (id: string, kind: Approval['kind'], job: string | null): Approval => ({
  id,
  kind,
  project: 'frostveil',
  runId: null,
  runNumber: null,
  step: null,
  job,
  assetName: null,
  service: null,
  detail: null,
  items: [],
  requestedAtMs: 0,
});

const hound = ask('rig-hound', 'rig-doctor', 'spiked-hound-v1');
const duchess = ask('rig-duchess', 'rig-doctor', 'duchess-vesperine-v1');
const send = ask('send', 'send-to-3d', 'spiked-hound-v1');

function waiting(approvals: Approval[]) {
  useAgentsStore.setState({ status: { approvals } as unknown as McpStatus });
}

const floating = () => renderHook(() => useFloatingApprovals()).result.current?.map(a => a.id);

afterEach(() => {
  useAgentsStore.setState({ status: null });
  useRigStore.setState({ record: null });
  useAppStore.setState({ view: 'board' });
});

describe('the floating approvals', () => {
  it('leave the rig doctor case on show to the Rig doctor, and float the rest', () => {
    waiting([hound, duchess, send]);
    useAppStore.setState({ view: 'inbox' });
    useRigStore.setState({ record: { id: 'spiked-hound-v1' } as RigRecord });
    expect(floating()).toEqual(['rig-duchess', 'send']);
  });

  it('float every rig doctor card in the Inbox list, with no case on show', () => {
    waiting([hound, duchess]);
    useAppStore.setState({ view: 'inbox' });
    expect(floating()).toEqual(['rig-hound', 'rig-duchess']);
  });

  it('float a case left open in the rig store once another view is on show', () => {
    waiting([hound]);
    useRigStore.setState({ record: { id: 'spiked-hound-v1' } as RigRecord });
    useAppStore.setState({ view: 'board' });
    expect(floating()).toEqual(['rig-hound']);
  });
});
