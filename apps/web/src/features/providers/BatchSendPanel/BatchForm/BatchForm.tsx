import { Button, HelpMark, Icon, InlineBanner, SegmentedControl, cn } from '@maquette/ui';
import {
  batchLabel,
  isCustom,
  itemDiff,
  itemMeshy,
  itemTripo,
  type BatchDraft,
} from '@/state/batch';
import { PROVIDER_LABEL, SERVICE_LABEL } from '@/state/providers';
import { formatCredits } from '@/state/credits';
import { CostEstimate } from '../../CostEstimate';
import { MeshyPresetFields, TripoPresetFields } from '../../PresetFields';
import { PromptField } from '../../PromptField';
import { SectionTitle } from '../../SectionTitle';
import { PROVIDERS, SERVICES } from '../../choices';
import { useBatchActions, useBatchEstimate, useBatchView } from './BatchForm.hooks';
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
  const service = draft.service;
  const api = draft.provider !== 'by-hand';
  const n = draft.items.length;
  const asking = confirming && api && !blocker;
  const assetsAside = `${n} ${n === 1 ? 'asset' : 'assets'}, one model each`;
  const items = draft.items.map(item => (
    <ItemRow
      key={item.asset}
      item={item}
      projectDir={projectDir}
      onVariant={variant => setItemVariant(item.asset, variant)}
      diff={isCustom(draft, item) ? itemDiff(draft, item, catalog) : null}
      open={open.includes(item.asset)}
      onToggle={() => toggle(item.asset)}
      onClear={() => setItemPreset(item.asset, null)}
      fields={
        service === 'tripo' ? (
          <TripoPresetFields
            compact
            preset={itemTripo(draft, item)}
            variant={item.variant}
            onChange={tripo => setItemPreset(item.asset, { tripo })}
          />
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
          <ul className="m-0 flex list-none flex-col p-0">{items}</ul>
        </section>

        <section className="flex flex-col gap-3">
          <SectionTitle
            aside={
              <button
                type="button"
                className="cursor-pointer text-meta text-fg-mute underline-offset-2 hover:text-fg hover:underline"
                onClick={reset}
              >
                Reset to the presets
              </button>
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
            <TripoPresetFields
              compact
              preset={draft.tripo[shown]}
              variant={shown}
              onChange={tripo => setPreset(shown, { tripo })}
            />
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

      <footer className="flex flex-col gap-2.5 border-t border-line px-5 py-3">
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
      </footer>
    </>
  );
}
