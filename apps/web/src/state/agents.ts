/** Words and numbers for the MCP server in Settings > Agents and the status bar. */
import type { Approval, CommandSource, McpStatus } from '@/ipc/client';

export interface ServerLine {
  tone: 'ok' | 'error' | 'off';
  text: string;
}

export function serverLine(status: McpStatus | null, enabled: boolean): ServerLine {
  if (status?.running) {
    return {
      tone: 'ok',
      text: `Listening on ${status.url}. Agents need the token below.`,
    };
  }
  if (enabled && status?.error) return { tone: 'error', text: status.error };
  if (enabled) return { tone: 'off', text: 'Starting the server' };
  return {
    tone: 'off',
    text: 'Off. Turn it on to let Claude Code and Codex use Maquette over MCP.',
  };
}

/** `14:02:31` in local time. */
export function clockTime(ms: number): string {
  const d = new Date(ms);
  const two = (n: number) => String(n).padStart(2, '0');
  return `${two(d.getHours())}:${two(d.getMinutes())}:${two(d.getSeconds())}`;
}

/** The last folder of a repo path, on either computer's separators. */
function repoFolder(repo: string | undefined): string {
  return repo?.split(/[\\/]/).filter(Boolean).pop() ?? 'this site';
}

/** Where a dev command came from, for the page preview card. */
export const COMMAND_SOURCE: Record<CommandSource, string> = {
  settings: 'Settings > Project',
  'site-config': "the site's maquette.site.json",
  'package-json': "the site's package.json",
};

/**
 * The approval's question: "Ship Frostveil, run 26?" with the project's
 * display name when known, "Send Guardian statue to Tripo?", or "Make 2
 * images for Harbour with OpenAI Images?".
 */
/**
 * Which approval cards are open. A new card opens only while no other is, so
 * several never bury the page. One answered does not open the next: its
 * Approve must not slide under the pointer that just clicked.
 */
export function settleOpen(
  open: Readonly<Record<string, boolean>>,
  ids: readonly string[]
): Readonly<Record<string, boolean>> {
  const next: Record<string, boolean> = {};
  let anyOpen = ids.some(id => open[id]);
  for (const id of ids) {
    const known = open[id];
    next[id] = known ?? !anyOpen;
    if (next[id]) anyOpen = true;
  }
  const same = ids.length === Object.keys(open).length && ids.every(id => open[id] === next[id]);
  return same ? open : next;
}

export function approvalTitle(approval: Approval, projectName?: string): string {
  if (approval.kind === 'image-round') {
    const n = approval.conceptJobs?.length ?? 0;
    const what = n === 1 ? '1 image' : `${n} images`;
    const where = approval.assetName ? ` for ${approval.assetName}` : '';
    return `Make ${what}${where} with OpenAI Images?`;
  }
  if (approval.kind === 'rest-pose') {
    return `Approve ${approval.assetName ?? 'the avatar'}'s new rest pose?`;
  }
  if (approval.kind === 'springs') {
    return `Add spring chains to ${approval.assetName ?? 'the avatar'}?`;
  }
  if (approval.kind === 'rig-doctor') {
    return `Add ${approval.assetName ?? 'the draft'}?`;
  }
  if (approval.kind === 'reweight') {
    return `Free ${approval.assetName ?? 'the avatar'}'s cloth and hair from the arms?`;
  }
  if (approval.kind === 'page-preview') {
    return `Run the dev command of ${repoFolder(approval.preview?.repo)}?`;
  }
  if (approval.kind === 'send-to-3d') {
    const n = approval.items.length;
    const what = n > 1 ? `${n} assets` : (approval.assetName ?? approval.items[0]?.assetName);
    return `Send ${what ?? 'this asset'} to ${approval.service ?? '3D'}?`;
  }
  return `Ship ${projectName ?? approval.project}, run ${approval.runNumber ?? '?'}?`;
}

/** "1 run waits", "2 sends wait", "3 requests wait" for approval. */
export function approvalsLabel(approvals: Approval[]): string {
  const n = approvals.length;
  const kinds = new Set(approvals.map(a => a.kind));
  const noun =
    kinds.size > 1
      ? 'request'
      : kinds.has('ship')
        ? 'run'
        : kinds.has('image-round')
          ? 'image round'
          : kinds.has('rest-pose')
            ? 'rest pose'
            : kinds.has('reweight')
              ? 'reweight'
              : kinds.has('springs')
                ? 'spring chain'
                : kinds.has('page-preview')
                  ? 'dev server'
                  : 'send';
  return `${n} ${noun}${n === 1 ? ' waits' : 's wait'} for approval`;
}

/** The status bar line for the server, or null when it is off. */
export function mcpStatusItem(
  status: McpStatus | null
): { label: string; dot: 'ok' | 'offline' | 'running' } | null {
  if (!status?.enabled) return null;
  if (status.approvals.length > 0) {
    return { label: approvalsLabel(status.approvals), dot: 'running' };
  }
  if (status.running) return { label: `MCP on 127.0.0.1:${status.port}`, dot: 'ok' };
  return { label: status.error ? 'MCP server stopped' : 'MCP starting', dot: 'offline' };
}
