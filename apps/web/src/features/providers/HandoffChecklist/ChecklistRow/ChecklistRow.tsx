import { Button, Icon, IconButton, StatusChip } from '@maquette/ui';
import type { ProviderJob } from '@/ipc/client';
import { shortPath } from '@/lib/format';
import { wantsModel } from '@/state/batch';
import { SERVICE_LABEL, jobChip } from '@/state/providers';
import { KIND_ICON } from '@/state/thumbs';
import { HandBackButtons } from '../../HandBackButtons';
import { HandoffView } from '../../HandoffView';
import { useChecklistRow } from './ChecklistRow.hooks';

export interface ChecklistRowProps {
  job: ProviderJob;
}

export default function ChecklistRow({ job }: ChecklistRowProps) {
  const { asset, thumb, openHandoff, expanded, toggleExpanded } = useChecklistRow(job);
  const chip = jobChip(job);
  const card = job.card;
  // The first rows of the card, and its rig, as the card says them.
  const shown = card ? [...card.rows.slice(0, 3), ...card.rows.filter(r => r.label === 'Rig')] : [];
  const rows = shown.map(r => `${r.label} ${r.value}`);
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
            tooltip="The settings to choose on the site, and the prompt to paste"
            onClick={toggleExpanded}
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
