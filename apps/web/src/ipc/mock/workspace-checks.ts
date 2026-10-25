/**
 * Mock mode's answers about workspace folders and configs: what a folder
 * holds (from its name, since there is no disk) and the checks a config
 * save runs, like the core's.
 */
import type { RootInspection, WorkspaceConfig } from '../client';
import { display, expand, fail } from './backend-options';
import { DEFAULT_ROOT, HOME } from './fixtures';

export function inspect(path: string): RootInspection {
  const full = expand(path.trim());
  const base = { path: full, display: display(full) };
  if (/existing/i.test(full)) return { ...base, state: 'workspace', entries: 4 };
  if (/Documents$|Desktop$|Downloads$/.test(full) || full === HOME) {
    return { ...base, state: 'not-empty', entries: 12 };
  }
  if (full === expand(DEFAULT_ROOT)) return { ...base, state: 'empty', entries: 0 };
  return { ...base, state: 'missing', entries: 0 };
}

export function validate(next: WorkspaceConfig) {
  const port = next.agents.mcpPort;
  if (port < 1024 || port > 65535) fail('invalid', 'The MCP port must be between 1024 and 65535.');
  if (!((next.budgets.sceneMb ?? 0) > 0))
    fail('invalid', 'The scene budget must be more than 0 MB and at most 1000 MB.');
}
