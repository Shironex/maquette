import { Button, InlineBanner, SegmentedControl, cn, type SegmentedOption } from '@maquette/ui';
import type { SplitSource } from '@/ipc/client';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { CostEstimate } from '../CostEstimate';
import { SectionTitle } from '../SectionTitle';
import { Sheet } from '@/features/common/Sheet';
import { formatK } from '@/lib/format';
import { PROVIDER_LABEL } from '@/state/providers';
import { SPLIT_WAYS, sourceLabel, type SplitWay } from '@/state/split';
import MeshyFields from './MeshyFields';
import TripoFields from './TripoFields';
import { noteCard } from './noteCard';
import { useSplitSheet, type SplitSheetModel } from './SplitSheet.hooks';

/** "v2 from Tripo · one mesh, 18.2k triangles, 4 materials" */
function metaLine(source: SplitSource): string {
  const from = source.madeBy ? ` from ${PROVIDER_LABEL[source.madeBy]}` : '';
  const shape =
    source.objects > 1 ? `${source.objects} meshes` : source.objects === 1 ? 'one mesh' : '';
  const numbers = [
    shape,
    `${formatK(source.triangles)} triangles`,
    `${source.materials} ${source.materials === 1 ? 'material' : 'materials'}`,
  ].filter(Boolean);
  return `${sourceLabel(source)}${from} · ${numbers.join(', ')}`;
}

/** Split a model into parts: free in Blender, on Tripo or on Meshy. */
export default function SplitSheet() {
  const s = useSplitSheet();
  if (!s.target) return null;
  const source = s.sheet?.source ?? null;
  const name = source?.assetName ?? 'the model';
  const ways: SegmentedOption<SplitWay>[] = SPLIT_WAYS.map(w =>
    w.value === 'meshy' && source?.meshyProblem
      ? { ...w, disabled: true, hint: source.meshyProblem }
      : w.value === 'blender' && !s.target?.inBlender
        ? { ...w, disabled: true, hint: 'Accept the model first: the free split is a fix-up.' }
        : w
  );
  const blocked = ways
    .filter(w => w.disabled)
    .map(w => (
      <p key={w.value} className="text-meta text-fg-mute">
        {w.hint}
      </p>
    ));
  return (
    <Sheet
      id="split-sheet"
      icon="stack"
      title={`Split ${name} into parts`}
      meta={source ? metaLine(source) : undefined}
      onClose={s.close}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        {s.error ? (
          <InlineBanner tone="danger" title="Could not read the model" detail={s.error} />
        ) : null}
        <section className="flex flex-col gap-2.5">
          <SectionTitle>Split with</SectionTitle>
          <SegmentedControl<SplitWay>
            label="Split with"
            tone="clay"
            block
            options={ways}
            value={s.way}
            onChange={s.setWay}
          />
          {blocked}
          {s.way !== 'blender' ? <KeyLine s={s} /> : null}
        </section>
        {source ? <WayFields s={s} source={source} /> : null}
      </div>
      <div className="flex flex-col gap-2.5 border-t border-line px-5 py-3">
        <p
          className={cn('truncate text-meta', s.blocker ? 'text-amber' : 'text-fg-mute')}
          title={s.blocker ?? s.summary}
        >
          {s.blocker ?? s.summary}
        </p>
        <div className="flex items-center justify-end gap-2">
          {s.way === 'blender' ? (
            <span className="text-meta font-medium text-fg">Free</span>
          ) : (
            <CostEstimate credits={s.credits} />
          )}
          <span className="mr-auto" />
          <Button variant="secondary" onClick={s.close}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon="stack"
            disabled={!source || Boolean(s.blocker) || s.sending}
            disabledReason={s.blocker ?? undefined}
            loading={s.sending}
            onClick={s.way === 'blender' ? () => void s.runInBlender() : s.confirm.ask}
          >
            {s.way === 'blender' ? 'Split in Blender' : `Split with ${PROVIDER_LABEL[s.way]}`}
          </Button>
        </div>
      </div>
      {s.confirm.open && s.way !== 'blender' ? (
        <ChoiceDialog
          title={`Split ${name} with ${PROVIDER_LABEL[s.way]}?`}
          onDismiss={s.confirm.keep}
          choices={[
            { label: 'Cancel', onClick: s.confirm.keep },
            {
              label: 'Yes, split',
              variant: 'primary',
              loading: s.sending,
              onClick: () => void s.send(),
            },
          ]}
        >
          <p>{s.question}</p>
        </ChoiceDialog>
      ) : null}
    </Sheet>
  );
}

/** The key the split spends with, or where to add one. */
function KeyLine({ s }: { s: SplitSheetModel }) {
  if (s.way === 'blender') return null;
  if (!s.key?.saved) {
    return (
      <InlineBanner
        tone="warning"
        title={`No ${PROVIDER_LABEL[s.way]} key yet`}
        detail="Keys live in the keychain. Add one in Settings > 3D providers, or split in Blender for free."
        actions={
          <Button size="sm" onClick={s.openSettings}>
            Open Settings
          </Button>
        }
      />
    );
  }
  return (
    <p className="flex items-start gap-2 text-meta text-fg-mute">
      <span className="mt-[5px] size-2 flex-none rounded-full bg-sage shadow-[0_0_0_3px_var(--color-sage-soft)]" />
      <span>
        Key ending in <code className="font-mono text-fg-dim">{s.key.tail}</code>, from the
        keychain.{' '}
        {s.way === 'tripo'
          ? 'This spends Tripo credits, and Tripo cannot cancel once it is sent.'
          : 'This spends Meshy credits. You can cancel while Meshy has not started.'}
      </span>
    </p>
  );
}

/** The chosen way's own settings. */
function WayFields({ s, source }: { s: SplitSheetModel; source: SplitSource }) {
  const next = 'the next version';
  if (s.way === 'tripo') {
    return <TripoFields source={source} value={s.tripo.value} onChange={s.tripo.set} next={next} />;
  }
  if (s.way === 'meshy') {
    return (
      <MeshyFields
        source={source}
        value={s.meshy.value}
        onChange={s.meshy.set}
        texture={s.texture}
      />
    );
  }
  return (
    <div className={noteCard}>
      <h4 className="font-semibold">Split into parts, in Blender</h4>
      <p className="text-meta text-fg-mute">
        Adds the Split into parts fix-up, by loose pieces: it runs headless on a copy, so the model
        file stays and the texture with it. Then name the parts in the Inspector.
      </p>
      <p className="text-meta text-fg-mute">
        Best when the pieces already sit apart. When they are welded into one shell, Tripo or Meshy
        find the parts.
      </p>
    </div>
  );
}
