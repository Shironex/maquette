/**
 * Pure helpers for editing a workspace config: section updates, the same
 * checks the Rust side runs, and the MCP snippets agents paste.
 */
import type { ModelOption, WorkspaceConfig } from '@/ipc/client';
import { presetProblems, type PresetKey } from './provider-presets';

type Section = Exclude<keyof WorkspaceConfig, 'version' | 'currentProject'>;

/** Returns a new config with `patch` merged into one section. */
export function patchSection<S extends Section>(
  config: WorkspaceConfig,
  section: S,
  patch: Partial<WorkspaceConfig[S]>
): WorkspaceConfig {
  return { ...config, [section]: { ...config[section], ...patch } };
}

export type ConfigField = PresetKey | 'sceneMb' | 'trisPerModel' | 'mcpPort' | 'layout';
export type ConfigErrors = Partial<Record<ConfigField, string>>;

const FOLDER_NAME = /^[^\\/:*?"<>|]+$/;

/** The model lists, for the labels in preset messages. */
export interface ModelLists {
  tripoModels: readonly ModelOption[];
  meshyModels: readonly ModelOption[];
}

export const NO_LISTS: ModelLists = { tripoModels: [], meshyModels: [] };

/** Mirrors the checks in the Rust `workspace::validate`, field by field. */
export function validateConfig(
  config: WorkspaceConfig,
  lists: ModelLists = NO_LISTS
): ConfigErrors {
  const errors: ConfigErrors = presetProblems(config, lists);
  const mb = config.budgets.sceneMb;
  if (mb === null || !Number.isFinite(mb) || mb <= 0 || mb > 1000) {
    errors.sceneMb = 'Use more than 0 MB and at most 1000 MB.';
  }
  if (!(config.budgets.trisPerModel > 0)) errors.trisPerModel = 'Use more than 0 triangles.';
  const port = config.agents.mcpPort;
  if (!Number.isInteger(port) || port < 1024 || port > 65535) {
    errors.mcpPort = 'Use a port between 1024 and 65535.';
  }
  const names = [config.layout.projects, config.layout.inbox, config.layout.cache];
  const plain = names.every(n => FOLDER_NAME.test(n) && n !== '.' && n !== '..');
  const unique = new Set(names.map(n => n.toLowerCase())).size === names.length;
  if (!plain || !unique) errors.layout = 'Use three different plain folder names.';
  return errors;
}

const FIELD_LABEL = {
  sceneMb: 'Scene target',
  trisPerModel: 'Tris per model',
  mcpPort: 'Localhost port',
  layout: 'Folder names',
} as const;

/** One line per problem for a dialog: the field, then what is wrong. */
export function describeErrors(errors: ConfigErrors): string[] {
  return (Object.entries(errors) as [ConfigField, string][]).map(([field, text]) => {
    const preset = /^(tripo|meshy)-(prop|character)$/.exec(field);
    const label = preset
      ? `${preset[1] === 'tripo' ? 'Tripo' : 'Meshy'} ${preset[2]} preset`
      : FIELD_LABEL[field as keyof typeof FIELD_LABEL];
    return `${label}: ${text}`;
  });
}

/**
 * The draft with every invalid field put back to its saved value, so the
 * valid edits next to it can still be kept.
 */
export function dropInvalid(
  draft: WorkspaceConfig,
  saved: WorkspaceConfig,
  errors: ConfigErrors
): WorkspaceConfig {
  const next = structuredClone(draft);
  for (const field of Object.keys(errors) as ConfigField[]) {
    if (field === 'sceneMb' || field === 'trisPerModel') {
      next.budgets[field] = saved.budgets[field] as never;
    } else if (field === 'mcpPort') {
      next.agents.mcpPort = saved.agents.mcpPort;
    } else if (field === 'layout') {
      next.layout = structuredClone(saved.layout);
    } else {
      const [service, variant] = field.split('-') as ['tripo' | 'meshy', 'prop' | 'character'];
      next.providers[service][variant] = structuredClone(
        saved.providers[service][variant]
      ) as never;
    }
  }
  return next;
}

export function hasErrors(errors: ConfigErrors): boolean {
  return Object.keys(errors).length > 0;
}

export function sameConfig(a: WorkspaceConfig, b: WorkspaceConfig): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

export type SaveStatus = 'saved' | 'pending' | 'saving' | 'invalid' | 'error';

/** What the "Changes save as you go" line says. */
export function saveStatus(
  draft: WorkspaceConfig,
  saved: WorkspaceConfig,
  { saving, failed }: { saving: boolean; failed: boolean }
): SaveStatus {
  if (hasErrors(validateConfig(draft))) return 'invalid';
  if (saving) return 'saving';
  if (failed) return 'error';
  return sameConfig(draft, saved) ? 'saved' : 'pending';
}

export const SAVE_STATUS_LABEL: Record<SaveStatus, string> = {
  saved: 'Changes save as you go',
  pending: 'Saving soon',
  saving: 'Saving',
  invalid: 'Fix the highlighted field to save',
  error: 'Could not save, retrying on the next change',
};

export function mcpUrl(port: number): string {
  return `http://127.0.0.1:${port}/mcp`;
}

export type McpAgent = 'claude' | 'codex';

/**
 * A working setup for Claude Code (a `claude mcp add` command) or Codex (a
 * `~/.codex/config.toml` table), with the server's bearer token in it. The
 * tool timeout gives run_pipeline and render_thumbnail room to wait.
 */
export function mcpSnippet(agent: McpAgent, port: number, token: string): string {
  const url = mcpUrl(port);
  if (agent === 'codex') {
    return [
      '[mcp_servers.maquette]',
      `url = "${url}"`,
      `http_headers = { Authorization = "Bearer ${token}" }`,
      'tool_timeout_sec = 150',
      '',
    ].join('\n');
  }
  return `claude mcp add --transport http maquette ${url} --header "Authorization: Bearer ${token}"\n`;
}

/** Shows the start and end of a token, enough to tell two apart. */
export function maskToken(token: string): string {
  return token.length > 12 ? `${token.slice(0, 6)}\u2026${token.slice(-4)}` : token;
}

/** Reads the port out of `4791` or `127.0.0.1:4791`; null when it is not a port. */
export function parsePort(text: string): number | null {
  const match = text.trim().match(/^(?:127\.0\.0\.1:|localhost:)?(\d{1,5})$/);
  return match ? Number(match[1]) : null;
}
