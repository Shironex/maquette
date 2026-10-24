/**
 * By hand: the settings card for the provider's web app, with a copy
 * button on every value, and how the model comes back.
 */
import { Button, Icon, IconButton } from '@maquette/ui';
import type { ProviderJob } from '@/ipc/client';
import { copyText } from '@/features/config/ConfigFields';
import { shortPath } from '@/lib/format';
import { useAppStore } from '@/state/app-store';
import { SERVICE_LABEL } from '@/state/providers';
import { useProvidersStore } from '@/state/providers-store';

function CopyButton({ text, what }: { text: string; what: string }) {
  const notify = useAppStore(s => s.notify);
  return (
    <IconButton
      icon="copy"
      size="sm"
      label={`Copy ${what}`}
      onClick={() =>
        void copyText(text).then(ok =>
          notify(
            ok
              ? { status: 'done', title: `${what} copied` }
              : {
                  status: 'failed',
                  title: 'Could not copy',
                  detail: 'The clipboard is not available here.',
                }
          )
        )
      }
    />
  );
}

function Step({ n, title, children }: { n: number; title: string; children?: React.ReactNode }) {
  return (
    <li className="flex gap-3">
      <span className="grid size-6 flex-none place-items-center rounded-full border-[1.5px] border-clay-line font-mono text-[11.5px] leading-none font-semibold text-clay-text">
        {n}
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-2 pt-0.5">
        <div className="font-semibold">{title}</div>
        {children}
      </div>
    </li>
  );
}

export function HandoffView({ job }: { job: ProviderJob }) {
  const openHandoff = useProvidersStore(s => s.openHandoff);
  const card = job.card;
  if (!card) return null;
  const service = SERVICE_LABEL[card.service];
  const folder = job.handoff ?? '';
  const waiting = job.state === 'waiting';
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-start gap-3 rounded-card border border-line bg-raised px-3.5 py-3">
        <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-clay-soft text-clay-text">
          <Icon name={waiting ? 'clock' : 'check'} size={17} />
        </span>
        <div className="min-w-0 flex-1">
          <div className="font-semibold">
            {waiting ? 'Waiting for your model' : 'The model is back'}
          </div>
          <p className="mt-0.5 text-meta text-fg-mute">
            No API call and no credits from me. You make the model in the {service} web app with
            these settings, then bring the GLB back.
          </p>
        </div>
      </div>

      <ol className="flex list-none flex-col gap-4 p-0">
        <Step n={1} title={`Open ${service}`}>
          <div className="flex items-center gap-2 rounded-control border border-line bg-app px-2.5 py-1.5">
            <code className="min-w-0 flex-1 truncate font-mono text-[12px] text-fg-dim">
              {card.site}
            </code>
            <CopyButton text={card.site} what="The link" />
          </div>
          <p className="text-meta text-fg-mute">Choose {card.mode} there.</p>
        </Step>

        <Step n={2} title="Upload the images">
          <div className="flex items-center gap-2 rounded-control border border-line bg-app px-2.5 py-1.5">
            <Icon name="folder" size={14} className="flex-none text-fg-mute" />
            <code
              className="min-w-0 flex-1 truncate font-mono text-[12px] text-fg-dim"
              title={folder}
            >
              {shortPath(folder, 4)}
            </code>
            <Button size="sm" icon="folder-open" onClick={() => void openHandoff(job.id)}>
              Open folder
            </Button>
          </div>
          <p className="font-mono text-[12px] text-fg-mute">{card.images.join(', ')}</p>
        </Step>

        <Step n={3} title="Use these settings">
          <dl className="m-0 flex flex-col rounded-card border border-line bg-raised">
            {card.rows.map(row => (
              <div
                key={row.label}
                className="grid grid-cols-[132px_1fr_auto] items-center gap-2 border-t border-line px-3 py-1.5 first:border-t-0"
              >
                <dt className="text-meta text-fg-mute">{row.label}</dt>
                <dd className="m-0 min-w-0">
                  <div className="truncate font-mono text-[12.5px]">{row.value}</div>
                  {row.note ? (
                    <div className="truncate text-[11.5px] text-fg-mute">{row.note}</div>
                  ) : null}
                </dd>
                <CopyButton text={row.value} what={row.label} />
              </div>
            ))}
          </dl>
          <p className="text-meta text-fg-mute">
            The same card is in the folder as settings.md and settings.json.
          </p>
        </Step>

        <Step n={4} title="Bring the GLB back">
          <p className="text-meta text-fg-dim">
            Drop it on the {job.assetName} card on the Board, or save it into the inbox as
          </p>
          <div className="flex items-center gap-2 rounded-control border border-line bg-app px-2.5 py-1.5">
            <code className="min-w-0 flex-1 truncate font-mono text-[12px]">{card.returnName}</code>
            <CopyButton text={card.returnName} what="The file name" />
          </div>
          <p className="text-meta text-fg-mute">
            Either way it links to {job.assetName} and the Inbox checks run.
          </p>
        </Step>
      </ol>
    </div>
  );
}
