import { describe, expect, it } from 'vitest';
import type { McpStatus } from '@/ipc/client';
import { approvalTitle, clockTime, mcpStatusItem, serverLine } from './agents';

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

const approval = {
  id: 'approval-1',
  runId: 'moon-island-run-26',
  project: 'moon-island',
  runNumber: 26,
  step: 'Ship',
  requestedAtMs: 0,
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
    expect(approvalTitle(approval, 'Moon Island')).toBe('Moon Island, run 26');
    expect(approvalTitle(approval)).toBe('moon-island, run 26');
  });
});
