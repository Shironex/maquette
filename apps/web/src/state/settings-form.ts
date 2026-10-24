/**
 * Pure helpers for editing a workspace config: section updates, the same
 * checks the Rust side runs, and the MCP snippets agents paste.
 */
import type { WorkspaceConfig } from '@/ipc/client';

type Section = Exclude<keyof WorkspaceConfig, 'version' | 'currentProject'>;

/** Returns a new config with `patch` merged into one section. */
export function patchSection<S extends Section>(
  config: WorkspaceConfig,
  section: S,
  patch: Partial<WorkspaceConfig[S]>
): WorkspaceConfig {
  return { ...config, [section]: { ...config[section], ...patch } };
}

export type ConfigField = 'model' | 'sceneMb' | 'trisPerModel' | 'quads' | 'mcpPort' | 'layout';
export type ConfigErrors = Partial<Record<ConfigField, string>>;

const FOLDER_NAME = /^[^\\/:*?"<>|]+$/;

/** Mirrors the checks in the Rust `workspace::validate`, field by field. */
export function validateConfig(config: WorkspaceConfig): ConfigErrors {
  const errors: ConfigErrors = {};
  if (!config.tripo.model.trim()) errors.model = 'Pick a Tripo model.';
  const mb = config.budgets.sceneMb;
  if (mb === null || !Number.isFinite(mb) || mb <= 0 || mb > 1000) {
    errors.sceneMb = 'Use more than 0 MB and at most 1000 MB.';
  }
  if (!(config.budgets.trisPerModel > 0)) errors.trisPerModel = 'Use more than 0 triangles.';
  if (!(config.tripo.character.quads > 0)) errors.quads = 'Use more than 0 quads.';
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

/** The snippet for Claude Code (`.mcp.json`) or Codex (`~/.codex/config.toml`). */
export function mcpSnippet(agent: 'claude' | 'codex', port: number): string {
  if (agent === 'codex') {
    return `[mcp_servers.maquette]\nurl = "${mcpUrl(port)}"\n`;
  }
  return `${JSON.stringify({ mcpServers: { maquette: { type: 'http', url: mcpUrl(port) } } }, null, 2)}\n`;
}

/** Reads the port out of `4791` or `127.0.0.1:4791`; null when it is not a port. */
export function parsePort(text: string): number | null {
  const match = text.trim().match(/^(?:127\.0\.0\.1:|localhost:)?(\d{1,5})$/);
  return match ? Number(match[1]) : null;
}
