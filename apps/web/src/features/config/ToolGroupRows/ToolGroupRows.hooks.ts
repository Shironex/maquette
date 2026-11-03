import { useEffect, useState } from 'react';
import { commands, errorMessage, type McpToolGroup, type WorkspaceConfig } from '@/ipc/client';
import { patchSection } from '@/state/settings-form';

/** The groups never change while the app runs, so one read serves every mount. */
let cached: McpToolGroup[] | null = null;

/** The server's tool groups with their counts; empty until read, or when the read fails. */
export function useToolGroups() {
  const [groups, setGroups] = useState<McpToolGroup[]>(cached ?? []);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (cached) return;
    let live = true;
    commands
      .mcpToolGroups()
      .then(read => {
        cached = read;
        if (live) setGroups(read);
      })
      .catch((err: unknown) => {
        if (live) setError(errorMessage(err));
      });
    return () => {
      live = false;
    };
  }, []);
  return { groups, error };
}

/** Test support: forget the read groups. */
export function forgetToolGroups() {
  cached = null;
}

/** Whether `group` is offered: always for the core groups, else unless turned off. */
export function isOn(group: McpToolGroup, off: readonly string[]) {
  return group.alwaysOn || !off.includes(group.id);
}

/** The config with `id` turned on or off; the order of the other ids stays. */
export function withGroup(config: WorkspaceConfig, id: string, on: boolean): WorkspaceConfig {
  const rules = config.agents.rules;
  const off = (rules.toolGroupsOff ?? []).filter(other => other !== id);
  return patchSection(config, 'agents', {
    rules: { ...rules, toolGroupsOff: on ? off : [...off, id] },
  });
}

/** How many tools the groups that are on hold, and how many there are. */
export function offeredCount(groups: readonly McpToolGroup[], off: readonly string[]) {
  const total = groups.reduce((sum, g) => sum + g.tools, 0);
  const offered = groups.filter(g => isOn(g, off)).reduce((sum, g) => sum + g.tools, 0);
  return { offered, total };
}
