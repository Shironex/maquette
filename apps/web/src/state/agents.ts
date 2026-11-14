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
 * Which approval cards are open. A new card opens only while no other is, so
 * several never bury the page, and one answered does not open the next. A
 * card can still open or move under the pointer, so this alone does not
 * protect its Approve from a stray click; the hold in `useApprovalStack` does.
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

/**
 * The approval's question: "Ship Frostveil, run 26?" with the project's
 * display name when known, "Send Guardian statue to Tripo?", or "Make 2
 * images for Harbour with OpenAI Images?".
 */
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
    return `Add the rig doctor fix as ${approval.assetName ?? 'a new version'}?`;
  }
  if (approval.kind === 'recipe-bake') {
    return `Add the recipe bake as ${approval.assetName ?? 'a new version'}?`;
  }
  if (approval.kind === 'recipe-save') {
    return `Save ${approval.assetName ?? 'the recipe'} to the recipe library?`;
  }
  if (approval.kind === 'grip-save') {
    return `Save a grip: ${approval.assetName ?? 'a hand pose'}`;
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

/**
 * "library/grips/scythe-staff-grip · 33 bones · r2 to r3" (or "· a new
 * grip") for a grip save approval; null for any other.
 */
export function gripSaveLine(approval: Approval): string | null {
  const ask = approval.recipeSave?.grip;
  if (approval.kind !== 'grip-save' || !ask) return null;
  const folder = ask.file.replace(/\/r\d+\.grip\.json$/, '');
  const bones = `${ask.bones.length} ${ask.bones.length === 1 ? 'bone' : 'bones'}`;
  const to = approval.recipeSave?.revision ?? 1;
  const revisions = ask.latest === null ? 'a new grip' : `r${ask.latest} to r${to}`;
  return [folder, bones, revisions].join(' · ');
}

/** What the status bar calls one waiting approval of each kind, and several. */
const APPROVAL_NOUN: Record<Approval['kind'], [one: string, many: string]> = {
  ship: ['run', 'runs'],
  'send-to-3d': ['send', 'sends'],
  'image-round': ['image round', 'image rounds'],
  'rest-pose': ['rest pose', 'rest poses'],
  reweight: ['reweight', 'reweights'],
  springs: ['spring chain', 'spring chains'],
  'rig-doctor': ['rig doctor fix', 'rig doctor fixes'],
  'recipe-bake': ['recipe bake', 'recipe bakes'],
  'recipe-save': ['recipe save', 'recipe saves'],
  'grip-save': ['grip save', 'grip saves'],
  'page-preview': ['dev server', 'dev servers'],
};

/**
 * "1 run waits", "2 sends wait", "1 run and 1 rig doctor fix wait" for
 * approval; three kinds or more are "4 requests wait".
 */
export function approvalsLabel(approvals: Approval[]): string {
  const n = approvals.length;
  const counts = new Map<Approval['kind'], number>();
  for (const a of approvals) counts.set(a.kind, (counts.get(a.kind) ?? 0) + 1);
  const named = (kind: Approval['kind'], count: number) => {
    const [one, many] = APPROVAL_NOUN[kind] ?? ['request', 'requests'];
    return `${count} ${count === 1 ? one : many}`;
  };
  const verb = n === 1 ? 'waits' : 'wait';
  if (counts.size > 2) return `${n} requests ${verb} for approval`;
  const parts = Array.from(counts, ([kind, count]) => named(kind, count));
  return `${parts.join(' and ')} ${verb} for approval`;
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
