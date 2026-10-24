/**
 * Every By hand send of the project in one list, grouped by batch: its
 * settings, its folder and whether the model is back. Opens after a By
 * hand batch, and from the 3D queue.
 */
import { useState } from 'react';
import { Button, Icon, IconButton, StatusChip, cn } from '@maquette/ui';
import type { ProviderJob } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatAgo, shortPath } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useAppStore } from '@/state/app-store';
import { backLabel, checklist, waitingByHand, wantsModel } from '@/state/batch';
import { projectFile } from '@/state/board';
import { SERVICE_LABEL, jobChip } from '@/state/providers';
import { useProvidersStore } from '@/state/providers-store';
import { KIND_ICON } from '@/state/thumbs';
import { HandBackButtons, HandoffView, siteLabel } from './HandoffView';
import { Sheet } from './Sheet';

export function HandoffChecklist() {
  const open = useProvidersStore(s => s.checklistOpen);
  const setOpen = useProvidersStore(s => s.setChecklistOpen);
  const jobs = useProvidersStore(s => s.jobs);
  const openSite = useProvidersStore(s => s.openSite);
  const checkDownloads = useProvidersStore(s => s.checkDownloads);
  const now = useNow(30_000);
  if (!open) return null;
  const groups = checklist(jobs);
  const waiting = waitingByHand(jobs);
  const services = [...new Set(waiting.map(j => j.service))];
  const close = () => setOpen(false);
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
        {(services.length ? services : (['tripo'] as const)).map(service => (
          <Button
            key={service}
            size="sm"
            icon="arrow-square-out"
            onClick={() => void openSite(service)}
          >
            {siteLabel(service)}
          </Button>
        ))}
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
          groups.map(group => (
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
                {group.jobs.map(job => (
                  <ChecklistRow key={job.id} job={job} />
                ))}
              </ul>
            </section>
          ))
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

function ChecklistRow({ job }: { job: ProviderJob }) {
  const workspace = useAppStore(s => s.workspace);
  const openHandoff = useProvidersStore(s => s.openHandoff);
  const [expanded, setExpanded] = useState(false);
  const asset = workspace?.project?.assets?.find(a => a.id === job.asset);
  const thumb = fileUrl(
    asset?.thumbnail && workspace ? projectFile(workspace, asset.thumbnail) : null
  );
  const chip = jobChip(job);
  const card = job.card;
  const rows = card?.rows.slice(0, 3).map(r => `${r.label} ${r.value}`) ?? [];
  const settings = card
    ? [`${SERVICE_LABEL[card.service]} ${card.mode}`, ...rows].join(' · ')
    : SERVICE_LABEL[job.service];
  return (
    <li className="flex flex-col gap-2 border-t border-line px-3 py-2.5 first:border-t-0">
      <div className="flex items-start gap-3">
        <span className="grid size-11 flex-none place-items-center overflow-hidden rounded-control bg-studio">
          {thumb ? (
            <img src={thumb} alt="" className="size-full object-cover" />
          ) : (
            <Icon name={asset ? KIND_ICON[asset.kind] : 'cube'} className="text-fg-mute" />
          )}
        </span>
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <div className="flex items-center gap-2">
            <span className="min-w-0 flex-1 truncate font-semibold">{job.assetName}</span>
            <StatusChip status={chip.status} label={chip.label} icon={chip.icon} />
          </div>
          <div className="truncate text-meta text-fg-dim" title={settings}>
            {settings}
          </div>
          {job.handoff ? (
            <div className="flex items-center gap-1">
              <code
                className="min-w-0 truncate font-mono text-[11.5px] text-fg-mute"
                title={job.handoff}
              >
                {shortPath(job.handoff, 3)}
              </code>
              <IconButton
                icon="folder-open"
                size="sm"
                label={`Open the ${job.assetName} folder`}
                onClick={() => void openHandoff(job.id)}
              />
            </div>
          ) : null}
        </div>
      </div>
      <div className="flex flex-wrap gap-1.5 pl-14">
        {wantsModel(job) ? <HandBackButtons job={job} size="sm" downloads={false} /> : null}
        {card ? (
          <Button
            size="sm"
            variant="ghost"
            icon="file-text"
            aria-expanded={expanded}
            onClick={() => setExpanded(v => !v)}
          >
            {expanded ? 'Hide the settings card' : 'Settings card'}
          </Button>
        ) : null}
      </div>
      {expanded ? (
        <div className="mt-1 ml-14 rounded-card border border-line bg-panel p-3">
          <HandoffView job={job} />
        </div>
      ) : null}
    </li>
  );
}
