import { describe, expect, it } from 'vitest';
import type { Approval, McpStatus } from '@/ipc/client';
import { approvalTitle, approvalsLabel, clockTime, mcpStatusItem, serverLine } from './agents';

const base: McpStatus = {
  enabled: true,
  running: true,
  port: 4791,
  url: 'http://127.0.0.1:4791/mcp',
  token: 'abc',
  error: null,
  askBeforeShipping: true,
  calls: [],
  approvals: [],
};

const approval: Approval = {
  id: 'approval-1',
  kind: 'ship',
  runId: 'frostveil-run-26',
  project: 'frostveil',
  runNumber: 26,
  step: 'Ship',
  job: null,
  assetName: null,
  service: null,
  detail: null,
  requestedAtMs: 0,
  items: [],
};

const send: Approval = {
  ...approval,
  id: 'approval-2',
  kind: 'send-to-3d',
  runId: null,
  runNumber: null,
  step: null,
  job: 'guardian-statue-2',
  assetName: 'Guardian statue',
  service: 'Tripo',
  detail: 'Tripo H3.1, 1 image',
};

describe('agents', () => {
  it('describes the server for each state', () => {
    expect(serverLine(base, true)).toEqual({
      tone: 'ok',
      text: 'Listening on http://127.0.0.1:4791/mcp. Agents need the token below.',
    });
    const failed = { ...base, running: false, error: 'Could not listen on 127.0.0.1:4791.' };
    expect(serverLine(failed, true).tone).toBe('error');
    expect(serverLine({ ...base, running: false }, false).tone).toBe('off');
    expect(serverLine(null, false).text).toContain('Off.');
  });

  it('puts approvals ahead of the port in the status bar', () => {
    expect(mcpStatusItem(base)).toEqual({ label: 'MCP on 127.0.0.1:4791', dot: 'ok' });
    expect(mcpStatusItem({ ...base, approvals: [approval] })).toEqual({
      label: '1 run waits for approval',
      dot: 'running',
    });
    expect(mcpStatusItem({ ...base, enabled: false })).toBeNull();
    expect(mcpStatusItem({ ...base, running: false, error: 'x' })?.dot).toBe('offline');
  });

  it('formats times and titles', () => {
    expect(clockTime(new Date(2026, 9, 24, 9, 5, 7).getTime())).toBe('09:05:07');
    expect(approvalTitle(approval, 'Frostveil')).toBe('Ship Frostveil, run 26?');
    expect(approvalTitle(approval)).toBe('Ship frostveil, run 26?');
    expect(approvalTitle(send)).toBe('Send Guardian statue to Tripo?');
    const batch: Approval = {
      ...send,
      job: null,
      assetName: null,
      detail: '2 models in total, one model per asset',
      items: [
        { job: 'lantern-post-1', assetName: 'Lantern post', detail: 'Tripo H3.1, 1 image' },
        { job: 'sigil-door-1', assetName: 'Sigil door', detail: 'Tripo H3.1, 1 image' },
      ],
    };
    expect(approvalTitle(batch)).toBe('Send 2 assets to Tripo?');
    expect(approvalTitle({ ...batch, items: batch.items.slice(0, 1) })).toBe(
      'Send Lantern post to Tripo?'
    );
  });

  it("asks to make an agent's OpenAI Images round", () => {
    const round: Approval = {
      ...send,
      id: 'approval-4',
      kind: 'image-round',
      job: null,
      assetName: 'Harbour',
      service: 'OpenAI Images',
      conceptJobs: ['harbour-01-x', 'harbour-02-x'],
      estimateUsd: 0.13,
    };
    expect(approvalTitle(round)).toBe('Make 2 images for Harbour with OpenAI Images?');
    expect(approvalTitle({ ...round, assetName: null, conceptJobs: ['harbour-01-x'] })).toBe(
      'Make 1 image with OpenAI Images?'
    );
    expect(approvalsLabel([round])).toBe('1 image round waits for approval');
  });

  it("asks about an agent's normalised rest pose", () => {
    const pose: Approval = {
      ...send,
      id: 'rest-pose-hero',
      kind: 'rest-pose',
      job: null,
      assetName: 'Hero',
      service: null,
      asset: 'hero',
      version: 2,
    };
    expect(approvalTitle(pose)).toBe("Approve Hero's new rest pose?");
    expect(approvalsLabel([pose])).toBe('1 rest pose waits for approval');
    const chains: Approval = { ...pose, id: 'springs-hero', kind: 'springs' };
    expect(approvalTitle(chains)).toBe('Add spring chains to Hero?');
    expect(approvalsLabel([chains, { ...chains, id: 'springs-2' }])).toBe(
      '2 spring chains wait for approval'
    );
  });

  it('counts approvals by kind', () => {
    expect(approvalsLabel([approval])).toBe('1 run waits for approval');
    expect(approvalsLabel([send, { ...send, id: 'approval-3' }])).toBe('2 sends wait for approval');
    expect(approvalsLabel([approval, send])).toBe('2 requests wait for approval');
  });
});
