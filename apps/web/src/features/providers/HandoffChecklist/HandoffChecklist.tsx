/**
 * Every By hand send of the project in one list, grouped by batch: its
 * settings, its folder and whether the model is back. Opens after a By
 * hand batch, and from the 3D queue.
 */
import { Button, cn } from '@maquette/ui';
import { formatAgo } from '@/lib/format';
import { backLabel } from '@/state/batch';
import { Sheet } from '../Sheet';
import { siteLabel } from '../siteLabel';
import { ChecklistRow } from './ChecklistRow';
import { useHandoffChecklist } from './HandoffChecklist.hooks';

export default function HandoffChecklist() {
  const { open, groups, waiting, services, openSite, checkDownloads, close, now } =
    useHandoffChecklist();
  if (!open) return null;
  const siteButtons = (services.length ? services : (['tripo'] as const)).map(service => (
    <Button key={service} size="sm" icon="arrow-square-out" onClick={() => void openSite(service)}>
      {siteLabel(service)}
    </Button>
  ));
  const sections = groups.map(group => {
    const rows = group.jobs.map(job => <ChecklistRow key={job.id} job={job} />);
    return (
      <section
        key={group.batch ?? group.jobs[0]?.id}
        aria-label={group.batch ? `Batch ${group.batch}` : group.jobs[0]?.assetName}
        className="mb-4 flex flex-col"
      >
        <header className="flex items-baseline gap-2 pb-1.5">
          <h3 className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
            {group.batch ? `Batch of ${group.jobs.length}` : 'Single send'}
          </h3>
          <span className="text-meta text-fg-mute">
            written {formatAgo(group.createdAtMs, now)}
          </span>
          <span
            className={cn(
              'ml-auto font-mono text-[11.5px] tabular-nums',
              group.back >= group.jobs.length ? 'text-sage' : 'text-fg-dim'
            )}
          >
            {backLabel(group)}
          </span>
        </header>
        <ul className="m-0 flex list-none flex-col rounded-card border border-line bg-raised p-0">
          {rows}
        </ul>
      </section>
    );
  });
  return (
    <Sheet
      id="checklist"
      icon="hand-pointing"
      title="By hand checklist"
      meta={
        waiting.length
          ? `${waiting.length} ${waiting.length === 1 ? 'model' : 'models'} to make · nothing spent`
          : 'Every model is back'
      }
      onClose={close}
      width={600}
    >
      <div className="flex flex-wrap items-center gap-2 border-b border-line px-5 py-2.5">
        {siteButtons}
        <Button
          size="sm"
          icon="download-simple"
          disabled={waiting.length === 0}
          onClick={() => void checkDownloads()}
        >
          Check Downloads
        </Button>
        <span className="ml-auto text-meta text-fg-mute">
          Drop a GLB on a card on the Board, or add it here.
        </span>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-5 py-3">
        {groups.length === 0 ? (
          <p className="py-8 text-center text-meta text-fg-mute">
            No By hand sends yet. Select assets on the Board and send them by hand.
          </p>
        ) : (
          sections
        )}
      </div>
      <footer className="flex justify-end gap-2 border-t border-line px-5 py-3">
        <Button variant="primary" onClick={close}>
          Done
        </Button>
      </footer>
    </Sheet>
  );
}
