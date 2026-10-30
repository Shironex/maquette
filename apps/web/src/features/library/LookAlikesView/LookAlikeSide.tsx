import { Button, InlineBanner, KeyValueList, NumberStepper, Toggle } from '@maquette/ui';
import type { LookAlikes } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { formatAgo } from '@/lib/format';
import { useLookAlikeSide } from './LookAlikesView.hooks';

export interface LookAlikeSideProps {
  report: LookAlikes;
  /** How many models the library holds. */
  models: number;
  /** Names of the models with no picture, for the warning. */
  unseen: string[];
}

const HEADING = 'text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase';

/** How look-alikes are found: the pictures, the threshold, the switches and the last scan. */
export default function LookAlikeSide({ report, models, unseen }: LookAlikeSideProps) {
  const { scanning, scan, saveSettings } = useLookAlikeSide();
  const { settings } = report;
  const seen = models - report.unseen.length;
  const took =
    report.tookMs < 1000 ? `${report.tookMs} ms` : `${Math.round(report.tookMs / 1000)} s`;
  return (
    <aside
      aria-label="How look-alikes are found"
      className="flex w-[320px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel p-[18px]"
    >
      <h2 className="text-panel-title">How it looks</h2>
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between">
          <b>Front picture of every model</b>
          <span className="font-mono text-fg-dim">{seen}</span>
        </div>
        <p className="text-meta text-fg-mute">
          The picture kept with each model. A new model brings its own when it is added.
        </p>
      </div>
      <div className="flex flex-col gap-1.5">
        <NumberStepper
          label="Look-alike above"
          value={settings.threshold ?? 0.9}
          min={0.5}
          max={0.99}
          step={0.01}
          format={v => v.toFixed(2)}
          onChange={threshold => void saveSettings({ ...settings, threshold })}
        />
        <p className="text-meta text-fg-mute">Outline, colour and box ratio, together.</p>
      </div>
      <Toggle
        label="Check when models are added"
        description="Adding a whole project stops on a look-alike and asks."
        checked={settings.checkOnAdd}
        onChange={checkOnAdd => void saveSettings({ ...settings, checkOnAdd })}
      />
      <Toggle
        label="Copy tags on merge"
        description="The merged model keeps the tags of both."
        checked={settings.copyTags}
        onChange={copyTags => void saveSettings({ ...settings, copyTags })}
      />
      <div className="h-px bg-line" />
      <div className={HEADING}>Last scan</div>
      <KeyValueList
        items={[
          { label: 'Scanned', value: formatAgo(Date.parse(report.scannedAt), clock.nowMs()) },
          { label: 'Groups open', value: String(report.pairs.length) },
          { label: 'Merged lately', value: String(report.merges.length) },
          { label: 'Took', value: took },
        ]}
      />
      <Button
        icon="arrows-clockwise"
        loading={scanning}
        disabled={scanning}
        tooltip="Compares every model's picture again; reads only"
        onClick={() => void scan()}
      >
        Scan again
      </Button>
      {unseen.length ? (
        <InlineBanner
          tone="warning"
          title={`${unseen.length} ${unseen.length === 1 ? 'model has' : 'models have'} no picture`}
          detail={`${unseen.slice(0, 3).join(', ')}${unseen.length > 3 ? ' and more' : ''}. The picture match skips them; names still pair.`}
        />
      ) : null}
    </aside>
  );
}
