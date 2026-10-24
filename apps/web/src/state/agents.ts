/** Words and numbers for the MCP server in Settings > Agents and the status bar. */
import type { Approval, McpStatus } from '@/ipc/client';

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

/**
 * The approval's question: "Ship Moon Island, run 26?" with the project's
 * display name when known, or "Send Stone lantern to Tripo?".
 */
export function approvalTitle(approval: Approval, projectName?: string): string {
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
  const noun = kinds.size > 1 ? 'request' : kinds.has('ship') ? 'run' : 'send';
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
