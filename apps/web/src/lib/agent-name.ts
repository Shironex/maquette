/**
 * An agent's name as people read it. Agents name themselves by their MCP
 * client's name, which is often a raw id ("claude-code", "rmcp"): known
 * clients read as their product, an SDK's default as "An agent", and any
 * other raw id as "An agent (mqtest)" so it can still be told apart. A name
 * that already reads as one (a title, with capitals or spaces) stays.
 */

const KNOWN: readonly (readonly [RegExp, string])[] = [
  [/claude[-_ ]?code/, 'Claude Code'],
  [/^claude/, 'Claude'],
  [/codex/, 'Codex'],
  [/cursor/, 'Cursor'],
  [/gemini/, 'Gemini CLI'],
  [/windsurf/, 'Windsurf'],
  [/vscode|visual[-_ ]studio[-_ ]code|copilot/, 'VS Code'],
  [/^zed\b/, 'Zed'],
];

/** The names an MCP SDK gives a client nobody named. */
const SDK_DEFAULTS = new Set(['rmcp', 'mcp', 'mcp-client', 'client', 'unknown', 'agent']);

const RAW_ID = /^[a-z0-9._-]+$/;

const SOMEONE = 'An agent';

/** `raw` (an MCP client's name, or a name an agent gave) as people read it. */
export function agentName(raw: string | null | undefined): string {
  const name = raw?.trim() ?? '';
  if (!name) return SOMEONE;
  const low = name.toLowerCase();
  const known = KNOWN.find(([pattern]) => pattern.test(low));
  if (known) return known[1];
  if (SDK_DEFAULTS.has(low)) return SOMEONE;
  return RAW_ID.test(name) ? `${SOMEONE} (${name})` : name;
}
