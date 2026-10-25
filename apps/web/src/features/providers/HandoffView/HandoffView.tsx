/**
 * By hand: the settings card for the provider's web app, with a copy
 * button on every value, buttons that open the web app, the folder and
 * each image, and how the model comes back.
 */
import { Button, Icon } from '@maquette/ui';
import type { ProviderJob } from '@/ipc/client';
import { shortPath } from '@/lib/format';
import { SERVICE_LABEL } from '@/state/providers';
import { HandBackButtons } from '../HandBackButtons';
import { siteLabel } from '../siteLabel';
import { CopyButton } from './CopyButton';
import { useFileManager, useHandoffActions } from './HandoffView.hooks';
import { Step } from './Step';

export interface HandoffViewProps {
  job: ProviderJob;
}

export default function HandoffView({ job }: HandoffViewProps) {
  const { openHandoff, openSite, revealImage } = useHandoffActions();
  const fileManager = useFileManager();
  const card = job.card;
  if (!card) return null;
  const service = SERVICE_LABEL[card.service];
  const folder = job.handoff ?? '';
  const waiting = job.state === 'waiting';
  const images = card.images.map(image => (
    <li
      key={image}
      className="flex items-center gap-1.5 rounded-control border border-line bg-raised py-0.5 pr-0.5 pl-2"
    >
      <span className="font-mono text-[12px] text-fg-dim">{image}</span>
      <Button
        size="sm"
        variant="ghost"
        icon="image"
        onClick={() => void revealImage(job.id, image)}
      >
        Reveal image
      </Button>
    </li>
  ));
  const rows = card.rows.map(row => (
    <div
      key={row.label}
      className="grid grid-cols-[132px_1fr_auto] items-center gap-2 border-t border-line px-3 py-1.5 first:border-t-0"
    >
      <dt className="text-meta text-fg-mute">{row.label}</dt>
      <dd className="m-0 min-w-0">
        <div className="truncate font-mono text-[12.5px]">{row.value}</div>
        {row.note ? <div className="truncate text-[11.5px] text-fg-mute">{row.note}</div> : null}
      </dd>
      <CopyButton text={row.value} what={row.label} />
    </div>
  ));
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
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant="primary"
              icon="arrow-square-out"
              onClick={() => void openSite(card.service)}
            >
              {siteLabel(card.service)}
            </Button>
            <code className="min-w-0 flex-1 truncate font-mono text-[12px] text-fg-mute">
              {card.site}
            </code>
            <CopyButton text={card.site} what="The link" />
          </div>
          <p className="text-meta text-fg-mute">
            It opens in your browser. Choose {card.mode} there.
          </p>
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
          <ul className="m-0 flex list-none flex-wrap gap-1.5 p-0">{images}</ul>
          <p className="text-meta text-fg-mute">
            Reveal shows the image in {fileManager}, ready to drag into the browser.
          </p>
        </Step>

        <Step n={3} title="Use these settings">
          <dl className="m-0 flex flex-col rounded-card border border-line bg-raised">{rows}</dl>
          <p className="text-meta text-fg-mute">
            The same card is in the folder as settings.md and settings.json.
          </p>
        </Step>

        <Step n={4} title="Bring the GLB back">
          {waiting ? (
            <div className="flex flex-wrap gap-2">
              <HandBackButtons job={job} size="sm" />
            </div>
          ) : null}
          <p className="text-meta text-fg-dim">
            Add model takes a .glb, or the .zip the web app downloads. Check Downloads looks for new
            models in your Downloads folder and asks before it takes one. You can also drop it on
            the {job.assetName} card on the Board, or save it into the inbox as
          </p>
          <div className="flex items-center gap-2 rounded-control border border-line bg-app px-2.5 py-1.5">
            <code className="min-w-0 flex-1 truncate font-mono text-[12px]">{card.returnName}</code>
            <CopyButton text={card.returnName} what="The file name" />
          </div>
          <p className="text-meta text-fg-mute">
            Every way copies the model into the inbox, leaves your file where it is, links it to{' '}
            {job.assetName} and runs the Inbox checks.
          </p>
        </Step>
      </ol>
    </div>
  );
}
