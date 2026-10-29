import { Button, Icon, InlineBanner, SegmentedControl, Tooltip, cn } from '@maquette/ui';
import type { SendOptions } from '@/ipc/client';
import {
  MAX_IMAGES,
  PROVIDER_LABEL,
  imagesNote,
  makeFront,
  toggleImage,
  type SendDraft,
} from '@/state/providers';
import { sentView, warningsLine } from '@/state/refs';
import { CostEstimate } from '../../CostEstimate';
import { MeshyPresetFields, TripoPresetFields } from '../../PresetFields';
import { PromptField } from '../../PromptField';
import { SectionTitle } from '../../SectionTitle';
import { StudioFields } from '../../StudioFields';
import { PROVIDERS, SERVICES, VARIANTS } from '../../choices';
import { EarlierSends } from './EarlierSends';
import { ImageChoice } from './ImageChoice';
import { ModelNote } from '../../ModelNote';
import { RefWarnings } from '../../RefWarnings';
import { useSendEstimate, useSendForm, useSendWarnings } from './SendForm.hooks';

export interface SendFormProps {
  options: SendOptions;
  draft: SendDraft;
}

export default function SendForm({ options, draft }: SendFormProps) {
  const {
    setDraft,
    reset,
    send,
    sending,
    close,
    openSettings,
    blocker,
    multiView,
    smart,
    summary,
    key,
    earlier,
    label,
    redo,
    studio,
  } = useSendForm(options, draft);
  const estimate = useSendEstimate(options, draft);
  const { warnings, note, fix } = useSendWarnings(options, draft, multiView);
  const tagged = (options.asset.refs ?? []).length > 0;
  const service = draft.service;
  const images = options.images.map(image => (
    <ImageChoice
      key={image.file}
      image={image}
      projectDir={options.projectDir}
      order={draft.images.indexOf(image.file)}
      view={tagged ? sentView(options.asset, draft.images, image.file) : undefined}
      warned={warnings.filter(w => w.file === image.file).length}
      full={draft.images.length >= MAX_IMAGES}
      onToggle={() => setDraft({ images: toggleImage(draft.images, image.file) })}
      onFront={() => setDraft({ images: makeFront(draft.images, image.file) })}
    />
  ));

  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        {redo ? (
          <InlineBanner
            title={redo.line}
            detail={`Prefilled from that send. Change what should differ; it goes out as the next version of ${redo.name}, and the earlier model stays in the Inbox so you can compare both and accept one.`}
          />
        ) : null}
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
                No API credits; your site plan pays. I write a handoff folder with the images and a
                settings card for the web app, and the asset waits for your GLB.
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
              {studio ? (
                <StudioFields
                  compact
                  value={draft.studio}
                  views={draft.images.length}
                  onChange={next => setDraft({ studio: next })}
                />
              ) : null}
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
                  tooltip="Closes this and opens Settings, where the key is added"
                  onClick={openSettings}
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
            <ul className="grid list-none grid-cols-4 gap-2 p-0">{images}</ul>
          )}
          <p className="text-meta text-fg-mute">
            {imagesNote(draft.provider, service, draft.images, multiView, smart)}
          </p>
          <ModelNote note={note} />
          <RefWarnings warnings={warnings} onFix={fix} />
        </section>

        <section className="flex flex-col gap-3">
          <SectionTitle
            aside={
              <Tooltip
                label="Reset the settings"
                detail="Puts every field back to what the preset says"
              >
                <button
                  type="button"
                  className="cursor-pointer text-meta text-fg-mute underline-offset-2 hover:text-fg hover:underline"
                  onClick={reset}
                >
                  Reset to the preset
                </button>
              </Tooltip>
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
              byHand={studio}
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
          {service === 'meshy' ? (
            <PromptField value={draft.prompt ?? ''} onChange={prompt => setDraft({ prompt })} />
          ) : null}
          <p className="text-meta text-fg-mute">
            Changes here are for this send only. The saved presets are in Settings &gt; 3D
            providers.
          </p>
        </section>

        {earlier.length > 0 ? <EarlierSends jobs={earlier} /> : null}
      </div>

      <div className="flex flex-col gap-2.5 border-t border-line px-5 py-3">
        <p
          className={cn('truncate text-meta', blocker ? 'text-amber' : 'text-fg-mute')}
          title={blocker ?? summary}
        >
          {blocker ?? summary}
        </p>
        <div className="flex items-center justify-end gap-2">
          <CostEstimate credits={estimate} />
          {warnings.length > 0 ? (
            <span className="inline-flex items-center gap-1 text-meta text-amber">
              <Icon name="warning" size={12} /> {warningsLine(warnings.length)}
            </span>
          ) : null}
          <span className="mr-auto" />
          <Button variant="secondary" onClick={close}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon={draft.provider === 'by-hand' ? 'folder' : 'cube'}
            loading={sending}
            disabled={Boolean(blocker) || sending}
            disabledReason={blocker ?? undefined}
            tooltip={
              draft.provider === 'by-hand'
                ? "Writes a folder with the images and settings to use on the provider's site; no credits are spent"
                : 'Sends the images to the provider; this spends the credits estimated here'
            }
            onClick={() => void send()}
          >
            {label}
          </Button>
        </div>
      </div>
    </>
  );
}
