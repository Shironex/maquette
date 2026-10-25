import type { ToolId, ToolReport } from '../client';
import { toolReports } from './fixtures';
import { fail, sleep, type Args } from './backend-options';

/** The toolchain commands; a located tool overrides what the scenario reports. */
export function createToolchainHandlers(
  scenario: string,
  probeDelay: number
): Record<string, (args: Args) => unknown> {
  const overrides = new Map<ToolId, string>();

  function tools(): ToolReport[] {
    return toolReports(scenario).map(r => {
      const path = overrides.get(r.id);
      return path
        ? {
            ...r,
            status: 'found',
            source: 'override',
            path,
            version: r.version ?? '1.0.0',
            detail: r.detail === 'not found on PATH' ? null : r.detail,
            hint: null,
          }
        : r;
    });
  }

  return {
    toolchain_detect: async () => {
      await sleep(probeDelay);
      return tools();
    },
    toolchain_locate: async ({ tool, path }) => {
      await sleep(probeDelay / 2);
      overrides.set(tool as ToolId, String(path));
      const found = tools().find(r => r.id === tool);
      if (!found) fail('not-found', `Could not find ${String(tool)}.`);
      return found;
    },
    toolchain_reset: ({ tool }) => {
      overrides.delete(tool as ToolId);
      return tools().find(r => r.id === tool);
    },
  };
}
