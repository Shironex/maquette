import { StatusChip } from '@maquette/ui';
import type { ProviderJob } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { jobChip, jobMeta, jobTitle } from '@/state/providers';
import { SectionTitle } from '../../../SectionTitle';

export interface EarlierSendsProps {
  jobs: ProviderJob[];
}

/** The asset's latest sends, under the form. */
export default function EarlierSends({ jobs }: EarlierSendsProps) {
  const rows = jobs.map(job => {
    const chip = jobChip(job);
    return (
      <li
        key={job.id}
        className="flex items-center gap-2.5 border-t border-line py-2 first:border-t-0"
      >
        <div className="min-w-0 flex-1">
          <div className="truncate font-medium">{jobTitle(job)}</div>
          <div className="truncate text-meta text-fg-mute">
            {job.message ?? jobMeta(job, clock.nowMs())}
          </div>
        </div>
        <StatusChip status={chip.status} label={chip.label} icon={chip.icon} />
      </li>
    );
  });
  return (
    <section className="flex flex-col gap-1.5">
      <SectionTitle>Earlier sends</SectionTitle>
      <ul className="m-0 flex list-none flex-col p-0">{rows}</ul>
    </section>
  );
}
