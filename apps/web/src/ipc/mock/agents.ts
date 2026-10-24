/**
 * A fake MCP server for mock mode. It "runs" whenever the workspace setting
 * is on; `?agents=active` adds an activity list from an agent session and a
 * run waiting at Ship for approval.
 */
import type { Approval, McpCall, McpStatus, WorkspaceConfig } from '../client';

type Args = Record<string, unknown>;

/** A fixed, obviously fake token, so screenshots never show a real one. */
export const MOCK_TOKEN = '7c1e9a4f0d3b58e2a6c4f19d8b0e7a35c2f6d1094be83a7f5c0d29e6b14a8f30';

export function agentCalls(now: number): McpCall[] {
  const rows: [string, string, McpCall['outcome'], string | null, number, number][] = [
    ['run_pipeline', 'moon-island, from compress', 'waiting', 'waiting for approval', 31_200, 12],
    ['get_pipeline', 'moon-island', 'ok', null, 18, 44],
    ['get_inbox_checks', 'character_tails_v3.glb', 'ok', null, 64, 72],
    ['accept_inbox_model', 'character_tails_v3.glb, moon-island', 'ok', null, 412, 95],
    ['list_inbox', '', 'ok', null, 22, 128],
    ['get_run', 'moon-island, number 25', 'ok', 'failed', 9, 160],
    [
      'render_thumbnail',
      'inbox/stone_lantern.glb',
      'error',
      'No Blender render for this model. It uses meshopt geometry.',
      31,
      204,
    ],
    ['list_projects', '', 'ok', null, 6, 233],
    ['connect', '', 'refused', 'A request came without the right token.', 0, 410],
  ];
  return rows.map(([tool, detail, outcome, message, durationMs, ago], i) => ({
    id: rows.length - i,
    atMs: now - ago * 1000,
    tool,
    detail,
    outcome,
    message,
    durationMs,
  }));
}

export function createFakeAgents({
  config,
  active,
  emit,
}: {
  config: () => WorkspaceConfig;
  /** `active`: an agent session is under way, with a run waiting at Ship. */
  active: boolean;
  emit?: (status: McpStatus) => void;
}) {
  const now = Date.now();
  let token: string | null = active ? MOCK_TOKEN : null;
  let calls: McpCall[] = active ? agentCalls(now) : [];
  let approvals: Approval[] = active
    ? [
        {
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
          requestedAtMs: now - 12_000,
        },
      ]
    : [];

  const status = (): McpStatus => {
    const agents = config().agents;
    if (agents.mcpEnabled && !token) token = MOCK_TOKEN;
    return {
      enabled: agents.mcpEnabled,
      running: agents.mcpEnabled,
      port: agents.mcpPort,
      url: `http://127.0.0.1:${agents.mcpPort}/mcp`,
      token,
      error: null,
      askBeforeShipping: agents.rules.askBeforeShipping,
      calls,
      approvals,
    };
  };

  const changed = () => emit?.(status());

  const handlers: Record<string, (args: Args) => unknown> = {
    mcp_status: () => status(),
    mcp_decide: ({ approvalId }) => {
      const before = approvals.length;
      approvals = approvals.filter(a => a.id !== approvalId);
      if (approvals.length === before) {
        throw { code: 'invalid', message: 'That request is no longer waiting for approval.' };
      }
      return status();
    },
    mcp_clear_activity: () => {
      calls = [];
      return status();
    },
    mcp_renew_token: () => {
      token = MOCK_TOKEN.split('').reverse().join('');
      return status();
    },
  };

  return { handlers, changed };
}
