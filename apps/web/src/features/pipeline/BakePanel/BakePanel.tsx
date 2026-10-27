import { Button, HelpMark, NumberField, Select, Toggle } from '@maquette/ui';
import type { BakePatch, BakeView } from '@/ipc/client';
import { costText, useBakePanel } from './BakePanel.hooks';

export interface BakePanelProps {
  bake: BakeView;
  busy: boolean;
  onSave: (patch: BakePatch) => Promise<boolean>;
}

const SIZES = ['256', '512', '1024', '2048', '4096'] as const;
const SIZE_OPTIONS = SIZES.map(value => ({ value, label: `${value} px` }));

function skippedText(count: number): string {
  if (count === 0) return 'Every model is baked.';
  return `${count} ${count === 1 ? 'model skips' : 'models skip'} the bake and keep their own lighting.`;
}

/** The Bake light step's lightmap size, samples and denoiser for this project. */
export default function BakePanel({ bake, busy, onSave }: BakePanelProps) {
  const { draft, dirty, update, save, saving, reset } = useBakePanel(bake, onSave);
  const blocked = busy ? 'Wait for the run to finish' : 'Nothing changed yet';
  const denoiseNote = bake.canDenoise
    ? 'OpenImageDenoise cleans the sample noise out of each lightmap.'
    : 'This project’s bake_light.py is older and cannot denoise. Update it from the starter banner.';

  return (
    <section
      aria-labelledby="bake-title"
      className="flex flex-none flex-wrap items-end gap-x-5 gap-y-3 rounded-card border border-line bg-panel px-4 py-3.5"
    >
      <div className="flex min-w-[180px] flex-1 flex-col">
        <div className="flex items-center gap-1.5">
          <h2 id="bake-title" className="text-[15px] leading-5 font-semibold">
            Baked light
          </h2>
          <HelpMark
            label="About the baked light"
            tip="The Bake light step renders the room's light into one lightmap per mesh with Cycles. A bigger lightmap is sharper; more samples are less noisy. Both cost time: bake time grows with the texels and the samples."
            side="bottom"
          />
        </div>
        <p className="text-meta mt-0.5 text-fg-mute">{skippedText(bake.skipped.length)}</p>
      </div>
      <Select
        label="Lightmap size"
        size="sm"
        options={SIZE_OPTIONS}
        value={String(draft.size) as (typeof SIZES)[number]}
        onChange={value => update({ size: Number(value) })}
        disabled={busy}
        fieldClassName="w-[120px]"
      />
      <NumberField
        label="Samples"
        size="sm"
        value={draft.samples}
        min={1}
        max={4096}
        onChange={samples => update({ samples: Math.round(samples) })}
        disabled={busy}
        fieldClassName="w-[96px]"
      />
      <div className="flex max-w-[260px] flex-col gap-1 pb-1">
        <Toggle
          label="Denoise"
          checked={draft.denoise}
          disabled={busy || !bake.canDenoise}
          onChange={denoise => update({ denoise })}
        />
        <p className="text-meta text-fg-mute">{denoiseNote}</p>
      </div>
      <div className="flex items-center gap-2 pb-0.5">
        <span className="text-meta text-fg-dim" aria-live="polite">
          {costText(draft)}
        </span>
        {dirty ? (
          <Button
            size="sm"
            variant="ghost"
            tooltip="Puts the settings back as last saved"
            onClick={reset}
          >
            Undo
          </Button>
        ) : null}
        <Button
          size="sm"
          variant="primary"
          loading={saving}
          disabled={!dirty || busy}
          disabledReason={blocked}
          tooltip="Saves these bake settings for the next run"
          onClick={() => void save()}
        >
          Apply to the bake
        </Button>
      </div>
    </section>
  );
}
