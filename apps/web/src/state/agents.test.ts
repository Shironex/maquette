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
  runId: 'moon-island-run-26',
  project: 'moon-island',
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
  job: 'stone-lantern-2',
  assetName: 'Stone lantern',
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
    expect(approvalTitle(approval, 'Moon Island')).toBe('Ship Moon Island, run 26?');
    expect(approvalTitle(approval)).toBe('Ship moon-island, run 26?');
    expect(approvalTitle(send)).toBe('Send Stone lantern to Tripo?');
    const batch: Approval = {
      ...send,
      job: null,
      assetName: null,
      detail: '2 models in total, one model per asset',
      items: [
        { job: 'desk-lamp-1', assetName: 'Desk lamp', detail: 'Tripo H3.1, 1 image' },
        { job: 'brush-cup-1', assetName: 'Brush cup', detail: 'Tripo H3.1, 1 image' },
      ],
    };
    expect(approvalTitle(batch)).toBe('Send 2 assets to Tripo?');
    expect(approvalTitle({ ...batch, items: batch.items.slice(0, 1) })).toBe(
      'Send Desk lamp to Tripo?'
    );
  });

  it('counts approvals by kind', () => {
    expect(approvalsLabel([approval])).toBe('1 run waits for approval');
    expect(approvalsLabel([send, { ...send, id: 'approval-3' }])).toBe('2 sends wait for approval');
    expect(approvalsLabel([approval, send])).toBe('2 requests wait for approval');
  });
});
