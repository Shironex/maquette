import { characters } from '@/state/avatar';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';

/**
 * The open avatar, for its name over the tabs, the tab shown, and a picker
 * over the asset's model versions that marks the one whose rest pose waits
 * for a yes.
 */
export function useAvatarTabs() {
  const view = useAvatarStore(s => s.view);
  const setVersion = useAvatarStore(s => s.setVersion);
  const tab = useAvatarStore(s => s.tab);
  const setTab = useAvatarStore(s => s.setTab);
  const project = useAppStore(s => s.workspace?.project);
  const approvals = useAgentsStore(s => s.status?.approvals);
  if (!view) return null;
  const waiting = new Set(
    (approvals ?? [])
      .filter(a => a.kind === 'rest-pose' && a.project === view.project && a.asset === view.asset)
      .map(a => a.version)
  );
  if (view.normalized?.state === 'pending') waiting.add(view.normalized.sourceVersion);
  const versions = characters(project).find(a => a.id === view.asset)?.versions ?? [];
  const options = versions.map(v => ({
    value: String(v.version),
    label: waiting.has(v.version) ? `v${v.version}, waits for you` : `v${v.version}`,
  }));
  return { view, options, waiting: waiting.size > 0, setVersion, tab, setTab };
}
