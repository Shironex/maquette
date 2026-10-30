import { CheckRow, type CheckStatus } from '@maquette/ui';
import type { FindingLevel } from '@/ipc/client';
import { Panel } from '@/features/common/Panel';
import { useReadinessPanel } from './ReadinessPanel.hooks';

const STATUS: Record<FindingLevel, CheckStatus> = {
  blocker: 'fail',
  warning: 'warning',
  note: 'pass',
};

/** What stands between the model and a VRM file, blockers first, none hidden. */
export default function ReadinessPanel() {
  const view = useReadinessPanel();
  if (!view) return null;
  const r = view.readiness;
  const blockers = r?.findings.filter(f => f.level === 'blocker').length ?? 0;
  const help = !r
    ? (view.readinessError ?? 'The model could not be read.')
    : r.ready
      ? 'A VRM file can be written from the model as it is.'
      : view.needsNormalize
        ? `${blockers} ${blockers === 1 ? 'blocker' : 'blockers'}. A build fixes the rest pose first, and you approve it.`
        : `${blockers} ${blockers === 1 ? 'blocker' : 'blockers'} to fix before a build writes.`;
  const rows = (r?.findings ?? []).map(f => (
    <CheckRow key={f.id} status={STATUS[f.level]} title={f.title} detail={f.detail ?? undefined} />
  ));
  return (
    <Panel id="avatar-readiness" title="VRM readiness" help={help}>
      {rows.length > 0 ? <div className="flex flex-col gap-2">{rows}</div> : null}
    </Panel>
  );
}
