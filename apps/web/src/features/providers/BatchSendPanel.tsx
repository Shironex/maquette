/**
 * Send several assets to 3D in one go: one provider and one set of
 * presets for all, a kind per asset, one model per asset. An API batch
 * asks once before it spends credits; a By hand batch writes one handoff
 * folder per asset and opens the checklist.
 */
import { useState } from 'react';
import { Button, Icon, InlineBanner, SegmentedControl, cn } from '@maquette/ui';
import type { PresetVariant } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { useAppStore } from '@/state/app-store';
import {
  batchBlocker,
  batchLabel,
  batchSummary,
  variantsUsed,
  type BatchDraft,
  type BatchItemDraft,
} from '@/state/batch';
import { inProject } from '@/state/lab';
import { meshyProblem, meshySummary, tripoProblem, tripoSummary } from '@/state/provider-presets';
import { PROVIDER_LABEL, SERVICE_LABEL } from '@/state/providers';
import { useProvidersStore } from '@/state/providers-store';
import { PROVIDERS, SERVICES, VARIANTS } from './choices';
import { MeshyPresetFields, TripoPresetFields, useCatalog } from './PresetFields';
import { SectionTitle, Sheet } from './Sheet';

const VARIANT_LABEL: Record<PresetVariant, string> = { prop: 'Props', character: 'Characters' };

export function BatchSendPanel() {
  const batch = useProvidersStore(s => s.batch);
  const close = useProvidersStore(s => s.closeBatch);
  if (!batch) return null;
  const n = batch.assets.length;
  return (
    <Sheet
      id="batch"
      icon="stack"
      title={`Send ${n} ${n === 1 ? 'asset' : 'assets'} to 3D`}
      meta={batch.draft ? batchSummary(batch.draft) : 'Reading the refs'}
      onClose={close}
      width={560}
    >
      {batch.error ? (
        <div className="p-5">
          <InlineBanner tone="danger" title="Could not open the batch" detail={batch.error} />
        </div>
      ) : batch.options && batch.draft ? (
        <BatchForm projectDir={batch.options.projectDir} draft={batch.draft} />
      ) : (
        <div className="flex flex-1 items-center justify-center gap-2 text-fg-mute">
          <Icon name="circle-notch" spin size={16} /> Reading the refs
        </div>
      )}
    </Sheet>
  );
}

function BatchForm({ projectDir, draft }: { projectDir: string; draft: BatchDraft }) {
  const setBatch = useProvidersStore(s => s.setBatch);
  const setItemVariant = useProvidersStore(s => s.setItemVariant);
  const setPreset = useProvidersStore(s => s.setBatchPreset);
  const reset = useProvidersStore(s => s.resetBatchPresets);
  const send = useProvidersStore(s => s.sendBatch);
  const sending = useProvidersStore(s => s.sending);
  const confirming = useProvidersStore(s => s.batch?.confirming ?? false);
  const close = useProvidersStore(s => s.closeBatch);
  const keys = useProvidersStore(s => s.keys);
  const setView = useAppStore(s => s.setView);
  const catalog = useCatalog();
  const used = variantsUsed(draft);
  const [tab, setTab] = useState<PresetVariant>(used[0] ?? 'prop');
  const shown: PresetVariant = used.includes(tab) ? tab : (used[0] ?? 'prop');
  const service = draft.service;
  const problem =
    used
      .map(v =>
        service === 'tripo'
          ? tripoProblem(draft.tripo[v], catalog.tripoModels)
          : meshyProblem(draft.meshy[v], catalog.meshyModels)
      )
      .find(Boolean) ?? null;
  const blocker = batchBlocker(draft, keys, problem);
  const key = keys.find(k => k.service === draft.provider);
  const summaryOf = (v: PresetVariant) =>
    service === 'tripo'
      ? tripoSummary(draft.tripo[v], catalog.tripoModels)
      : meshySummary(draft.meshy[v], catalog.meshyModels);
  const api = draft.provider !== 'by-hand';
  const n = draft.items.length;

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
            onChange={provider => setBatch({ provider })}
          />
          {draft.provider === 'by-hand' ? (
            <div className="flex flex-col gap-2.5 rounded-card border border-line bg-raised px-3 py-2.5">
              <p className="text-meta text-fg-dim">
                No API and nothing spent. I write one handoff folder per asset, each with its images
                and a settings card, and list them all in a checklist while you make the models.
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

        <section className="flex flex-col gap-1.5">
          <SectionTitle aside={`${n} ${n === 1 ? 'asset' : 'assets'}, one model each`}>
            Assets
          </SectionTitle>
          <ul className="m-0 flex list-none flex-col p-0">
            {draft.items.map(item => (
              <ItemRow
                key={item.asset}
                item={item}
                projectDir={projectDir}
                onVariant={variant => setItemVariant(item.asset, variant)}
              />
            ))}
          </ul>
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
            providers.
          </p>
        </section>
      </div>

      <footer className="flex flex-col gap-2.5 border-t border-line px-5 py-3">
        {confirming && api && !blocker ? (
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
                {n} {n === 1 ? 'task' : 'tasks'} with{' '}
                {used.map(v => `${VARIANT_LABEL[v].toLowerCase()}: ${summaryOf(v)}`).join('; ')}.
                {draft.provider === 'tripo'
                  ? ' Tripo cannot cancel a task once it is sent.'
                  : ' You can cancel each one while Meshy has not started it.'}
              </p>
            </div>
          </div>
        ) : (
          <p
            className={cn('truncate text-meta', blocker ? 'text-amber' : 'text-fg-mute')}
            title={blocker ?? used.map(v => `${VARIANT_LABEL[v]}: ${summaryOf(v)}`).join('\n')}
          >
            {blocker ?? used.map(v => `${VARIANT_LABEL[v]}: ${summaryOf(v)}`).join(' · ')}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={close}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon={api ? 'cube' : 'folder'}
            loading={sending}
            disabled={Boolean(blocker) || sending}
            onClick={() => void send()}
          >
            {confirming && api ? `Yes, send ${n}` : batchLabel(draft)}
          </Button>
        </div>
      </footer>
    </>
  );
}

function ItemRow({
  item,
  projectDir,
  onVariant,
}: {
  item: BatchItemDraft;
  projectDir: string;
  onVariant: (variant: PresetVariant) => void;
}) {
  const thumb = fileUrl(item.thumbnail ? inProject(projectDir, item.thumbnail) : null);
  const n = item.images.length;
  return (
    <li className="flex items-center gap-3 border-t border-line py-2 first:border-t-0">
      <span className="grid size-10 flex-none place-items-center overflow-hidden rounded-control bg-studio">
        {thumb ? (
          <img src={thumb} alt="" className="size-full object-cover" />
        ) : (
          <Icon name="image" className="text-fg-mute" />
        )}
      </span>
      <div className="min-w-0 flex-1">
        <div className="truncate font-medium">{item.name}</div>
        <div className={cn('truncate text-meta', n === 0 ? 'text-amber' : 'text-fg-mute')}>
          {n === 0
            ? 'No refs yet'
            : `${n} ${n === 1 ? 'image' : 'images'}: ${item.images.map(i => i.slice(i.lastIndexOf('/') + 1)).join(', ')}`}
        </div>
      </div>
      <SegmentedControl
        label={`${item.name} kind`}
        options={VARIANTS}
        value={item.variant}
        onChange={onVariant}
      />
    </li>
  );
}
