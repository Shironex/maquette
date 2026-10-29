import { Button, HelpMark, Icon, InlineBanner, SegmentedControl, Tooltip, cn } from '@maquette/ui';
import {
  batchLabel,
  isCustom,
  itemDiff,
  itemMeshy,
  itemStudio,
  itemTripo,
  studioBatch,
  type BatchDraft,
} from '@/state/batch';
import { PROVIDER_LABEL, SERVICE_LABEL } from '@/state/providers';
import { formatCredits } from '@/state/credits';
import { CostEstimate } from '../../CostEstimate';
import { MeshyPresetFields, TripoPresetFields } from '../../PresetFields';
import { PromptField } from '../../PromptField';
import { SectionTitle } from '../../SectionTitle';
import { StudioFields } from '../../StudioFields';
import { PROVIDERS, SERVICES } from '../../choices';
import {
  useBatchActions,
  useBatchEstimate,
  useBatchView,
  useBatchWarnings,
} from './BatchForm.hooks';
import { ItemRow } from './ItemRow';
import { VARIANT_LABEL } from './variantLabel';

export interface BatchFormProps {
  projectDir: string;
  draft: BatchDraft;
  /** A redo from the Inbox: each asset starts from its earlier send. */
  redo?: boolean;
}

export default function BatchForm({ projectDir, draft, redo = false }: BatchFormProps) {
  const {
    setBatch,
    setItemVariant,
    setPreset,
    setItemPreset,
    setItemPrompt,
    reset,
    send,
    sending,
    confirming,
    close,
    openSettings,
  } = useBatchActions();
  const estimate = useBatchEstimate(draft);
  const { catalog, used, shown, setTab, blocker, key, lines, open, toggle } = useBatchView(draft);
  const refs = useBatchWarnings(draft);
  const service = draft.service;
  const api = draft.provider !== 'by-hand';
  const studio = studioBatch(draft);
  const n = draft.items.length;
  const asking = confirming && api && !blocker;
  const assetsAside = `${n} ${n === 1 ? 'asset' : 'assets'}, one model each`;
  const shownItems = draft.items.filter(i => !refs.only || refs.byAsset.get(i.asset)?.length);
  const items = shownItems.map(item => (
    <ItemRow
      key={item.asset}
      item={item}
      projectDir={projectDir}
      warnings={refs.known ? (refs.byAsset.get(item.asset) ?? []) : null}
      modelNote={refs.notes.get(item.asset) ?? null}
      onFix={(w, f) => refs.fix(item.asset, w, f)}
      onVariant={variant => setItemVariant(item.asset, variant)}
      diff={isCustom(draft, item) ? itemDiff(draft, item, catalog) : null}
      open={open.includes(item.asset)}
      onToggle={() => toggle(item.asset)}
      onClear={() => setItemPreset(item.asset, null)}
      fields={
        service === 'tripo' ? (
          <>
            {studio ? (
              <StudioFields
                compact
                name={item.name}
                value={itemStudio(draft, item)}
                views={item.images.length}
                onChange={next => setItemPreset(item.asset, { studio: next })}
              />
            ) : null}
            <TripoPresetFields
              compact
              byHand={studio}
              preset={itemTripo(draft, item)}
              variant={item.variant}
              onChange={tripo => setItemPreset(item.asset, { tripo })}
            />
          </>
        ) : (
          <>
            <MeshyPresetFields
              compact
              preset={itemMeshy(draft, item)}
              variant={item.variant}
              onChange={meshy => setItemPreset(item.asset, { meshy })}
            />
            <PromptField
              name={item.name}
              rows={2}
              value={item.prompt ?? ''}
              onChange={prompt => setItemPrompt(item.asset, prompt)}
            />
          </>
        )
      }
    />
  ));

  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        {redo ? (
          <InlineBanner
            title={`Redo of ${n} ${n === 1 ? 'model' : 'models'} from the Inbox`}
            detail="Each asset starts from the send it came from: its images, kind and settings. Each goes out as its asset's next version; the earlier models stay in the Inbox to compare."
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
            onChange={provider => setBatch({ provider })}
          />
          {draft.provider === 'by-hand' ? (
            <div className="flex flex-col gap-2.5 rounded-card border border-line bg-raised px-3 py-2.5">
              <p className="text-meta text-fg-dim">
                No API credits; your site plan pays. I write one handoff folder per asset, each with
                its images and a settings card, and list them all in a checklist while you make the
                models.
              </p>
              <div className="flex items-center justify-between gap-3">
                <span className="text-fg-dim">Settings cards for</span>
                <SegmentedControl
                  label="Settings cards for"
                  options={SERVICES}
                  value={draft.service}
                  onChange={next => setBatch({ service: next })}
                />
              </div>
            </div>
          ) : key?.saved ? (
            <p className="flex items-start gap-2 text-meta text-fg-mute">
              <span className="mt-[5px] size-2 flex-none rounded-full bg-sage shadow-[0_0_0_3px_var(--color-sage-soft)]" />
              <span>
                Key ending in <code className="font-mono text-fg-dim">{key.tail}</code>, from the
                keychain. One task per asset, so {n} {n === 1 ? 'task spends' : 'tasks spend'}{' '}
                {PROVIDER_LABEL[draft.provider]} credits.
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

        <section className="flex flex-col gap-1.5">
          <SectionTitle
            aside={
              <span className="inline-flex items-center gap-1.5">
                {assetsAside}
                <HelpMark
                  label="About settings per asset"
                  tip="The sliders button on a row gives that asset its own settings. Only the fields you change stay with it; the rest follow the batch."
                  side="left"
                />
              </span>
            }
          >
            Assets
          </SectionTitle>
          {refs.warned > 0 ? (
            <div className="flex items-center gap-2 rounded-card border border-amber/40 bg-amber-soft/40 px-3 py-2 text-meta">
              <Icon name="warning" size={14} className="flex-none text-amber" />
              <span className="flex-1">
                {refs.warned} of {n} assets {refs.warned === 1 ? 'has' : 'have'} ref warnings. They
                still go; fix them first to spend the credits on better refs.
              </span>
              <Tooltip
                label={refs.only ? 'Show every asset' : 'Show only the assets with warnings'}
              >
                <button
                  type="button"
                  aria-pressed={refs.only}
                  className="flex-none cursor-pointer text-fg-dim underline-offset-2 hover:text-fg hover:underline"
                  onClick={() => refs.setOnly(!refs.only)}
                >
                  {refs.only ? 'Show all' : 'Show only these'}
                </button>
              </Tooltip>
            </div>
          ) : null}
          <ul className="m-0 flex list-none flex-col p-0">{items}</ul>
        </section>

        <section className="flex flex-col gap-3">
          <SectionTitle
            aside={
              <Tooltip
                label="Reset the batch settings"
                detail="Puts every field back to what the presets say, for all assets in the batch"
              >
                <button
                  type="button"
                  className="cursor-pointer text-meta text-fg-mute underline-offset-2 hover:text-fg hover:underline"
                  onClick={reset}
                >
                  Reset to the presets
                </button>
              </Tooltip>
            }
          >
            Settings for this batch
          </SectionTitle>
          {used.length > 1 ? (
            <div className="flex items-center justify-between gap-3">
              <span className="text-fg-dim">Preset to edit</span>
              <SegmentedControl
                label="Preset to edit"
                options={used.map(v => ({
                  value: v,
                  label: `${VARIANT_LABEL[v]} (${draft.items.filter(i => i.variant === v).length})`,
                }))}
                value={shown}
                onChange={setTab}
              />
            </div>
          ) : (
            <p className="text-meta text-fg-dim">
              Every asset uses the {shown} preset of {SERVICE_LABEL[service]}.
            </p>
          )}
          {service === 'tripo' ? (
            <>
              {studio ? (
                <StudioFields
                  compact
                  value={draft.studio[shown]}
                  views={Math.max(
                    0,
                    ...draft.items.filter(i => i.variant === shown).map(i => i.images.length)
                  )}
                  onChange={next => setPreset(shown, { studio: next })}
                />
              ) : null}
              <TripoPresetFields
                compact
                byHand={studio}
                preset={draft.tripo[shown]}
                variant={shown}
                onChange={tripo => setPreset(shown, { tripo })}
              />
            </>
          ) : (
            <MeshyPresetFields
              compact
              preset={draft.meshy[shown]}
              variant={shown}
              onChange={meshy => setPreset(shown, { meshy })}
            />
          )}
          <p className="text-meta text-fg-mute">
            Changes here are for this batch only. The saved presets are in Settings &gt; 3D
            providers. Each asset can change its own in the list above.
          </p>
        </section>
      </div>

      <div className="flex flex-col gap-2.5 border-t border-line px-5 py-3">
        {asking ? (
          <div
            role="alert"
            className="flex items-start gap-2.5 rounded-card border border-amber/40 bg-amber-soft/40 px-3 py-2.5"
          >
            <Icon name="warning" size={16} className="mt-0.5 flex-none text-amber" />
            <div className="min-w-0 flex-1">
              <div className="font-semibold">
                Send {n} {n === 1 ? 'asset' : 'assets'} to {PROVIDER_LABEL[draft.provider]}?
              </div>
              <p className="text-meta text-fg-dim">
                {n} {n === 1 ? 'task' : 'tasks'} with {lines.join('; ')}.
                {estimate !== null ? ` About ${formatCredits(estimate)} credits in all.` : ''}
                {refs.warned > 0
                  ? ` ${refs.warned} ${refs.warned === 1 ? 'asset goes' : 'assets go'} with ref warnings.`
                  : ''}
                {draft.provider === 'tripo'
                  ? ' Tripo cannot cancel a task once it is sent.'
                  : ' You can cancel each one while Meshy has not started it.'}
              </p>
            </div>
          </div>
        ) : (
          <p
            className={cn('truncate text-meta', blocker ? 'text-amber' : 'text-fg-mute')}
            title={blocker ?? lines.join('\n')}
          >
            {blocker ?? lines.join(' · ')}
          </p>
        )}
        <div className="flex items-center justify-end gap-2">
          <CostEstimate credits={estimate} className="mr-auto" />
          <Button variant="secondary" onClick={close}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon={api ? 'cube' : 'folder'}
            loading={sending}
            disabled={Boolean(blocker) || sending}
            disabledReason={blocker ?? undefined}
            tooltip={
              api
                ? 'Sends every image set in this batch to the provider; this spends the credits estimated here'
                : "Writes a handoff folder per asset to use on the provider's site; no credits are spent"
            }
            onClick={() => void send()}
          >
            {confirming && api ? `Yes, send ${n}` : batchLabel(draft)}
          </Button>
        </div>
      </div>
    </>
  );
}
