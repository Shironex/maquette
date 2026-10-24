/**
 * Send to 3D: pick the provider, the images and this send's settings,
 * then send. Opens from a picked concept in the Lab or an asset's card.
 */
import { Button, Icon, InlineBanner, SegmentedControl, StatusChip, cn } from '@maquette/ui';
import type { RefImage, SendOptions } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { deepEqual } from '@/lib/deepEqual';
import { formatBytes } from '@/lib/format';
import { useAppStore } from '@/state/app-store';
import { inProject } from '@/state/lab';
import {
  isSmartTopology,
  meshyProblem,
  meshySummary,
  tripoProblem,
  tripoSummary,
} from '@/state/provider-presets';
import {
  MAX_IMAGES,
  PROVIDER_LABEL,
  SERVICE_LABEL,
  VIEW_NAMES,
  imagesNote,
  jobChip,
  jobMeta,
  jobTitle,
  makeFront,
  sendBlocker,
  sortJobs,
  toggleImage,
  type SendDraft,
} from '@/state/providers';
import { useProvidersStore } from '@/state/providers-store';
import { HandoffView } from './HandoffView';
import { MeshyPresetFields, TripoPresetFields, useCatalog } from './PresetFields';
import { PROVIDERS, SERVICES, VARIANTS } from './choices';
import { SectionTitle, Sheet } from './Sheet';

/** The sheet itself, shown while the store has a panel open. */
export function SendPanel() {
  const panel = useProvidersStore(s => s.panel);
  const close = useProvidersStore(s => s.closePanel);
  if (!panel) return null;
  const name = panel.options?.asset.name ?? panel.handoff?.assetName ?? 'Asset';
  const handoff = panel.handoff;
  return (
    <Sheet
      id="send"
      icon={handoff ? 'hand-pointing' : 'cube'}
      title={handoff ? `${name}, by hand` : `Send ${name} to 3D`}
      meta={
        handoff
          ? `${handoff.id} · ${SERVICE_LABEL[handoff.service]} settings`
          : panel.options
            ? `${panel.options.asset.kind} · ${panel.options.images.length} ${panel.options.images.length === 1 ? 'ref' : 'refs'} in the project`
            : 'Reading the refs'
      }
      onClose={close}
      dirty={Boolean(panel.draft && !deepEqual(panel.draft, panel.initial))}
    >
      {handoff ? (
        <>
          <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
            <HandoffView job={handoff} />
          </div>
          <footer className="flex justify-end gap-2 border-t border-line px-5 py-3">
            <Button variant="primary" onClick={close}>
              Done
            </Button>
          </footer>
        </>
      ) : panel.error ? (
        <div className="p-5">
          <InlineBanner tone="danger" title="Could not open Send to 3D" detail={panel.error} />
        </div>
      ) : panel.options && panel.draft ? (
        <SendForm options={panel.options} draft={panel.draft} />
      ) : (
        <div className="flex flex-1 items-center justify-center gap-2 text-fg-mute">
          <Icon name="circle-notch" spin size={16} /> Reading the refs
        </div>
      )}
    </Sheet>
  );
}

function SendForm({ options, draft }: { options: SendOptions; draft: SendDraft }) {
  const setDraft = useProvidersStore(s => s.setDraft);
  const reset = useProvidersStore(s => s.resetDraft);
  const send = useProvidersStore(s => s.send);
  const sending = useProvidersStore(s => s.sending);
  const close = useProvidersStore(s => s.closePanel);
  const keys = useProvidersStore(s => s.keys);
  const allJobs = useProvidersStore(s => s.jobs);
  const setView = useAppStore(s => s.setView);
  const catalog = useCatalog();
  const service = draft.service;
  const problem =
    service === 'tripo'
      ? tripoProblem(draft.tripo, catalog.tripoModels)
      : meshyProblem(draft.meshy, catalog.meshyModels);
  const blocker = sendBlocker(draft.provider, keys, draft.images, problem);
  const multiView = service === 'tripo' ? draft.tripo.multiView : draft.meshy.multiView;
  const smart = service === 'meshy' && isSmartTopology(draft.meshy.aiModel);
  const summary =
    service === 'tripo'
      ? tripoSummary(draft.tripo, catalog.tripoModels)
      : meshySummary(draft.meshy, catalog.meshyModels);
  const key = keys.find(k => k.service === draft.provider);
  const earlier = sortJobs(allJobs.filter(j => j.asset === options.asset.id)).slice(0, 4);
  const label =
    draft.provider === 'by-hand'
      ? 'Write the handoff folder'
      : `Send to ${PROVIDER_LABEL[draft.provider]}`;

  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        <section className="flex flex-col gap-2.5">
          <SectionTitle>Where</SectionTitle>
          <SegmentedControl
            label="Provider"
            tone="clay"
            block
            options={PROVIDERS}
            value={draft.provider}
            onChange={provider => setDraft({ provider })}
          />
          {draft.provider === 'by-hand' ? (
            <div className="flex flex-col gap-2.5 rounded-card border border-line bg-raised px-3 py-2.5">
              <p className="text-meta text-fg-dim">
                No API and nothing spent. I write a handoff folder with the images and a settings
                card for the web app, and the asset waits for your GLB.
              </p>
              <div className="flex items-center justify-between gap-3">
                <span className="text-fg-dim">Settings card for</span>
                <SegmentedControl
                  label="Settings card for"
                  options={SERVICES}
                  value={draft.service}
                  onChange={next => setDraft({ service: next })}
                />
              </div>
            </div>
          ) : key?.saved ? (
            <p className="flex items-start gap-2 text-meta text-fg-mute">
              <span className="mt-[5px] size-2 flex-none rounded-full bg-sage shadow-[0_0_0_3px_var(--color-sage-soft)]" />
              <span>
                Key ending in <code className="font-mono text-fg-dim">{key.tail}</code>, from the
                keychain.{' '}
                {draft.provider === 'tripo'
                  ? 'This spends Tripo credits, and Tripo cannot cancel once it is sent.'
                  : 'This spends Meshy credits. You can cancel while Meshy has not started.'}
              </span>
            </p>
          ) : (
            <InlineBanner
              tone="warning"
              title={`No ${PROVIDER_LABEL[draft.provider]} key yet`}
              detail="Keys live in the keychain. Add one in Settings > 3D providers, or send by hand."
              actions={
                <Button
                  size="sm"
                  onClick={() => {
                    close();
                    setView('settings');
                  }}
                >
                  Open Settings
                </Button>
              }
            />
          )}
        </section>

        <section className="flex flex-col gap-2.5">
          <SectionTitle aside={`${draft.images.length} of ${MAX_IMAGES}`}>Images</SectionTitle>
          {options.images.length === 0 ? (
            <p className="rounded-card border border-dashed border-line-strong px-3 py-4 text-center text-meta text-fg-mute">
              No refs in this project yet. Send picks from the Lab, or add images to refs/.
            </p>
          ) : (
            <ul className="grid list-none grid-cols-4 gap-2 p-0">
              {options.images.map(image => (
                <ImageChoice
                  key={image.file}
                  image={image}
                  projectDir={options.projectDir}
                  order={draft.images.indexOf(image.file)}
                  full={draft.images.length >= MAX_IMAGES}
                  onToggle={() => setDraft({ images: toggleImage(draft.images, image.file) })}
                  onFront={() => setDraft({ images: makeFront(draft.images, image.file) })}
                />
              ))}
            </ul>
          )}
          <p className="text-meta text-fg-mute">
            {imagesNote(draft.provider, service, draft.images, multiView, smart)}
          </p>
        </section>

        <section className="flex flex-col gap-3">
          <SectionTitle
            aside={
              <button
                type="button"
                className="cursor-pointer text-meta text-fg-mute underline-offset-2 hover:text-fg hover:underline"
                onClick={reset}
              >
                Reset to the preset
              </button>
            }
          >
            Settings for this send
          </SectionTitle>
          <div className="flex items-center justify-between gap-3">
            <span className="text-fg-dim">Preset</span>
            <SegmentedControl
              label="Preset"
              options={VARIANTS}
              value={draft.variant}
              onChange={variant => setDraft({ variant })}
            />
          </div>
          {service === 'tripo' ? (
            <TripoPresetFields
              compact
              preset={draft.tripo}
              variant={draft.variant}
              onChange={tripo => setDraft({ tripo })}
            />
          ) : (
            <MeshyPresetFields
              compact
              preset={draft.meshy}
              variant={draft.variant}
              onChange={meshy => setDraft({ meshy })}
            />
          )}
          <p className="text-meta text-fg-mute">
            Changes here are for this send only. The saved presets are in Settings &gt; 3D
            providers.
          </p>
        </section>

        {earlier.length > 0 ? (
          <section className="flex flex-col gap-1.5">
            <SectionTitle>Earlier sends</SectionTitle>
            <ul className="m-0 flex list-none flex-col p-0">
              {earlier.map(job => {
                const chip = jobChip(job);
                return (
                  <li
                    key={job.id}
                    className="flex items-center gap-2.5 border-t border-line py-2 first:border-t-0"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="truncate font-medium">{jobTitle(job)}</div>
                      <div className="truncate text-meta text-fg-mute">
                        {job.message ?? jobMeta(job, Date.now())}
                      </div>
                    </div>
                    <StatusChip status={chip.status} label={chip.label} icon={chip.icon} />
                  </li>
                );
              })}
            </ul>
          </section>
        ) : null}
      </div>

      <footer className="flex flex-col gap-2.5 border-t border-line px-5 py-3">
        <p
          className={cn('truncate text-meta', blocker ? 'text-amber' : 'text-fg-mute')}
          title={blocker ?? summary}
        >
          {blocker ?? summary}
        </p>
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={close}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon={draft.provider === 'by-hand' ? 'folder' : 'cube'}
            loading={sending}
            disabled={Boolean(blocker) || sending}
            onClick={() => void send()}
          >
            {label}
          </Button>
        </div>
      </footer>
    </>
  );
}

function ImageChoice({
  image,
  projectDir,
  order,
  full,
  onToggle,
  onFront,
}: {
  image: RefImage;
  projectDir: string;
  order: number;
  full: boolean;
  onToggle: () => void;
  onFront: () => void;
}) {
  const url = fileUrl(inProject(projectDir, image.file));
  const on = order >= 0;
  return (
    <li className="group relative">
      <button
        type="button"
        aria-pressed={on}
        disabled={!on && full}
        onClick={onToggle}
        title={`${image.name}${image.bytes ? `, ${formatBytes(image.bytes)}` : ''}`}
        className={cn(
          'relative block aspect-square w-full cursor-pointer overflow-hidden rounded-[8px] border bg-studio transition-colors duration-120 disabled:cursor-not-allowed disabled:opacity-40',
          on ? 'border-clay ring-2 ring-clay/35' : 'border-line hover:border-line-strong'
        )}
      >
        {url ? (
          <img src={url} alt="" className="size-full object-cover" draggable={false} />
        ) : (
          <Icon name="image" className="text-fg-mute" />
        )}
        {on ? (
          <span className="absolute top-1 left-1 inline-flex h-[18px] items-center gap-1 rounded-pill bg-clay px-1.5 text-[10.5px] font-semibold text-clay-ink">
            {order + 1} {VIEW_NAMES[order]}
          </span>
        ) : null}
      </button>
      <div className="mt-1 truncate font-mono text-[11px] text-fg-mute">{image.name}</div>
      {on && order > 0 ? (
        <button
          type="button"
          onClick={onFront}
          className="absolute top-1 right-1 hidden h-[18px] cursor-pointer items-center rounded-pill bg-panel/90 px-1.5 text-[10.5px] font-medium text-fg group-focus-within:inline-flex group-hover:inline-flex"
        >
          Make front
        </button>
      ) : null}
    </li>
  );
}
