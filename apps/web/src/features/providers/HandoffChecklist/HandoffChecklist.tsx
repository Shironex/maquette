/**
 * Every By hand send of the project in one list, grouped by batch: its
 * settings, its folder and whether the model is back. One of the lists of
 * the Sends panel.
 */
import { Button, cn } from '@maquette/ui';
import { formatAgo } from '@/lib/format';
import { backLabel, liveJobs } from '@/state/batch';
import { siteLabel } from '../siteLabel';
import { ChecklistRow } from './ChecklistRow';
import { useHandoffChecklist } from './HandoffChecklist.hooks';

export default function HandoffChecklist() {
  const { groups, waiting, services, openSite, checkDownloads, now } = useHandoffChecklist();
  const siteButtons = (services.length ? services : (['tripo'] as const)).map(service => (
    <Button
      key={service}
      size="sm"
      icon="arrow-square-out"
      tooltip={`Opens ${siteLabel(service)} in your browser to make the models`}
      onClick={() => void openSite(service)}
    >
      {siteLabel(service)}
    </Button>
  ));
  const sections = groups.map(group => {
    const live = liveJobs(group).length;
    const stopped = group.jobs.length - live;
    const rows = group.jobs.map(job => <ChecklistRow key={job.id} job={job} />);
    return (
      <section
        key={group.batch ?? group.jobs[0]?.id}
        aria-label={group.batch ? `Batch ${group.batch}` : group.jobs[0]?.assetName}
        className="mb-4 flex flex-col"
      >
        <div className="flex items-baseline gap-2 pb-1.5">
          <h3 className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
            {group.batch ? `Batch of ${live}` : 'Single send'}
          </h3>
          <span className="text-meta text-fg-mute">
            written {formatAgo(group.createdAtMs, now)}
            {stopped > 0 ? `, ${stopped} stopped` : ''}
          </span>
          <span
            className={cn(
              'ml-auto font-mono text-[11.5px] tabular-nums',
              group.back >= live ? 'text-sage' : 'text-fg-dim'
            )}
          >
            {backLabel(group)}
          </span>
        </div>
        <ul className="m-0 flex list-none flex-col rounded-card border border-line bg-raised p-0">
          {rows}
        </ul>
      </section>
    );
  });
  return (
    <>
      <div className="flex flex-wrap items-center gap-2 border-b border-line px-5 py-2.5">
        {siteButtons}
        <Button
          size="sm"
          icon="download-simple"
          disabled={waiting.length === 0}
          disabledReason="No model is waiting"
          tooltip="Looks in your Downloads folder for the models you made"
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
    </>
  );
}
