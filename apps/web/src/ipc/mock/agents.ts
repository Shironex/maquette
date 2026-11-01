/**
 * A fake MCP server for mock mode. It "runs" whenever the workspace setting
 * is on; `?agents=active` adds an activity list from an agent session, a
 * run waiting at Ship for approval and a send to Tripo waiting the same way.
 * `?agents=batch` asks for three sends under one approval instead, and
 * `?agents=images` a round of two Lab images on OpenAI Images (the jobs
 * `?lab=approval` holds).
 */
import type { Approval, McpCall, McpStatus, RigRecord, WorkspaceConfig } from '../client';
import { createRigHandlers } from './create-rig-handlers';
import { clock } from '@/lib/clock';

type Args = Record<string, unknown>;

/** A fixed, obviously fake token, so screenshots never show a real one. */
export const MOCK_TOKEN = '7c1e9a4f0d3b58e2a6c4f19d8b0e7a35c2f6d1094be83a7f5c0d29e6b14a8f30';

export function agentCalls(now: number): McpCall[] {
  const rows: [string, string, McpCall['outcome'], string | null, number, number][] = [
    [
      'send_to_3d',
      'frostveil, asset rune-obelisk, provider tripo',
      'waiting',
      'waiting for approval',
      140,
      40,
    ],
    ['run_pipeline', 'frostveil, from compress', 'waiting', 'waiting for approval', 31_200, 12],
    ['get_pipeline', 'frostveil', 'ok', null, 18, 44],
    ['get_inbox_checks', 'elsheria_hair_v3.glb', 'ok', null, 64, 72],
    ['accept_inbox_model', 'elsheria_hair_v3.glb, frostveil', 'ok', null, 412, 95],
    ['list_inbox', '', 'ok', null, 22, 128],
    ['get_run', 'frostveil, number 25', 'ok', 'failed', 9, 160],
    [
      'render_thumbnail',
      'inbox/guardian_statue.glb',
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
  scene = null,
  rig: rigOptions,
  emit,
  onSendDecision,
  games,
  avatars,
}: {
  config: () => WorkspaceConfig;
  /** `active`: an agent session is under way, with a run waiting at Ship. */
  active: boolean;
  /**
   * `batch`: the send waiting is a batch of three. `images`: an OpenAI Images
   * round waits instead of it. `preview`: only a page preview waits, the first
   * start of a repo an agent set.
   */
  scene?: string | null;
  emit?: (status: McpStatus) => void;
  /** A send to 3D an agent asked for was approved or denied. */
  onSendDecision?: (job: string, approve: boolean) => void;
  /** Ship approvals runs of game projects ask for, whoever started them. */
  games?: { approvals: () => Approval[]; decide: (a: Approval, approve: boolean) => void };
  /** Rest pose approvals agents' avatar builds wait on; only the Avatar view answers them. */
  avatars?: { approvals: () => Approval[] };
  /** What an agent's rig doctor case waits for; only the rig doctor view adds it. */
  rig?: { state: string | null; pace: number; emit: (record: RigRecord) => void };
}) {
  const batch = scene === 'batch';
  const images = scene === 'images';
  const preview = scene === 'preview';
  const rig = rigOptions
    ? createRigHandlers(rigOptions.state, rigOptions.pace, rigOptions.emit)
    : null;
  const now = clock.nowMs();
  let token: string | null = active ? MOCK_TOKEN : null;
  let calls: McpCall[] = active ? agentCalls(now) : [];
  let approvals: Approval[] = active
    ? [
        {
          id: 'approval-1',
          kind: 'ship',
          runId: 'frostveil-run-26',
          project: 'frostveil',
          runNumber: 26,
          step: 'Ship',
          job: null,
          assetName: null,
          service: null,
          detail: 'Writes to /Users/you/code/frostveil-site',
          requestedAtMs: now - 12_000,
          items: [],
        },
        batch
          ? {
              id: 'approval-2',
              kind: 'send-to-3d',
              runId: null,
              project: 'frostveil',
              runNumber: null,
              step: null,
              job: null,
              assetName: null,
              service: 'Tripo',
              detail: '3 models in total, about 90 credits',
              estimate: 90,
              requestedAtMs: now - 40_000,
              items: [
                {
                  job: 'rune-obelisk-2',
                  assetName: 'Rune obelisk',
                  detail: 'Tripo H3.1, 1 image',
                  estimate: 30,
                },
                {
                  job: 'veil-gate-2',
                  assetName: 'Veil gate',
                  detail: 'Tripo H3.1, 1 image',
                  estimate: 30,
                },
                {
                  job: 'guardian-statues-2',
                  assetName: 'Guardian statues',
                  detail: 'Tripo H3.1, 1 image',
                  estimate: 30,
                },
              ],
            }
          : {
              id: 'approval-2',
              kind: 'send-to-3d',
              runId: null,
              project: 'frostveil',
              runNumber: null,
              step: null,
              job: 'rune-obelisk-2',
              assetName: 'Rune obelisk',
              service: 'Tripo',
              detail: 'Tripo H3.1, 1 image',
              estimate: 30,
              requestedAtMs: now - 40_000,
              items: [],
            },
      ]
    : [];

  if (images && approvals[0]) {
    approvals = [
      approvals[0],
      {
        id: 'approval-3',
        kind: 'image-round',
        runId: null,
        project: approvals[0].project,
        runNumber: null,
        step: null,
        job: null,
        assetName: 'Frozen lake arena',
        service: 'OpenAI Images',
        detail: '2 images at medium quality, about $0.13. $3.42 of $10 spent this month.',
        estimate: null,
        conceptJobs: ['frozen-lake-arena-07-mock', 'frozen-lake-arena-08-mock'],
        estimateUsd: 0.134,
        requestedAtMs: now - 25_000,
        items: [],
      },
    ];
  }

  if (preview) {
    approvals = [
      {
        id: 'approval-4',
        kind: 'page-preview',
        runId: null,
        project: 'frostveil',
        runNumber: null,
        step: null,
        job: null,
        assetName: null,
        service: null,
        detail: null,
        requestedAtMs: now - 8_000,
        items: [],
        preview: {
          repo: '/Users/you/code/frostveil-site',
          command: 'pnpm run dev --port {port}',
          source: 'package-json',
          scriptRuns: 'astro dev',
        },
      },
    ];
  }

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
      approvals: [
        ...approvals,
        ...(games?.approvals() ?? []),
        ...(avatars?.approvals() ?? []),
        ...(rig?.approvals() ?? []),
      ],
    };
  };

  const changed = () => emit?.(status());

  const handlers: Record<string, (args: Args) => unknown> = {
    ...rig?.handlers,
    mcp_status: () => status(),
    mcp_decide: ({ approvalId, approve }) => {
      const game = games?.approvals().find(a => a.id === approvalId);
      if (game) {
        games?.decide(game, Boolean(approve));
        return status();
      }
      const found = approvals.find(a => a.id === approvalId);
      if (!found) {
        throw { code: 'invalid', message: 'That request is no longer waiting for approval.' };
      }
      rig?.deny(approvalId as string);
      approvals = approvals.filter(a => a.id !== approvalId);
      const jobs = found.items.length ? found.items.map(i => i.job) : found.job ? [found.job] : [];
      for (const job of jobs) onSendDecision?.(job, Boolean(approve));
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

  return {
    handlers,
    changed,
    /** True while a page preview card holds `project`'s dev server back. */
    previewWaits: (project: string) =>
      approvals.some(a => a.kind === 'page-preview' && a.project === project),
  };
}
