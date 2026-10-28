import { BudgetMeter, NumberField, Select, cn } from '@maquette/ui';
import type { ImageBackend, ImageQuality, WorkspaceConfig } from '@/ipc/client';
import { Panel } from '@/features/common/Panel';
import { KeyRowView } from '@/features/providers/KeyFields';
import {
  BACKEND_LABEL,
  KIND_LABEL,
  QUALITY_LABEL,
  formatUsd,
  perImage,
  type ImageJobKind,
} from '@/state/image-api';
import { ACTIVE } from '../sections';
import { useImagesSection } from './ImagesSection.hooks';

const KINDS: readonly ImageJobKind[] = ['concepts', 'objectRefs', 'otherViews', 'cleanBackground'];
const BACKENDS: readonly ImageBackend[] = ['codex', 'openai-images'];
const QUALITIES: readonly ImageQuality[] = ['low', 'medium', 'high'];

export interface ImagesSectionProps {
  active: boolean;
  config: WorkspaceConfig;
  onChange: (config: WorkspaceConfig) => void;
}

/** Settings > Image generation: Codex or OpenAI's Image API per kind of Lab image. */
export default function ImagesSection({ active, config, onChange }: ImagesSectionProps) {
  const { settings, status, set, setBackend, key } = useImagesSection(config, onChange);
  const backends = BACKENDS.map(b => ({ value: b, label: BACKEND_LABEL[b] }));
  const pickers = KINDS.map(kind => (
    <Select<ImageBackend>
      key={kind}
      label={KIND_LABEL[kind]}
      options={backends}
      value={settings.backends[kind]}
      onChange={backend => setBackend(kind, backend)}
    />
  ));
  const spent = status?.monthSpentUsd ?? 0;
  const quality = QUALITIES.map(q => ({
    value: q,
    label: status
      ? `${QUALITY_LABEL[q]}, about ${formatUsd(perImage(status, q))}`
      : QUALITY_LABEL[q],
  }));

  return (
    <Panel
      id="settings-images"
      title="Image generation"
      help="Codex or OpenAI's Image API for each kind of Lab image, and what the API may spend"
      className={cn('gap-3 px-4 py-3.5', active && ACTIVE)}
    >
      <div className="grid grid-cols-2 gap-2.5">{pickers}</div>
      <div className="h-px bg-line" />
      <KeyRowView
        label="OpenAI"
        where="platform.openai.com, API keys"
        testTip="Lists the models the key can use, which costs nothing"
        {...key}
      />
      <div className="grid grid-cols-2 gap-2.5">
        <Select<ImageQuality>
          label="Quality"
          options={quality}
          value={settings.quality}
          onChange={q => set({ quality: q })}
        />
        <NumberField
          label="Monthly cap"
          unit="USD"
          value={settings.monthlyCapUsd}
          min={0}
          max={10000}
          format={n => n.toFixed(0)}
          onChange={cap => set({ monthlyCapUsd: Math.round(cap) })}
        />
      </div>
      <BudgetMeter
        label="Spent this month"
        value={spent}
        target={settings.monthlyCapUsd}
        unit=""
        format={formatUsd}
      />
      <p className="text-meta text-fg-mute">
        Codex makes every image unless a kind is set to OpenAI Images. Those rounds start without
        asking, each image goes in the workspace&apos;s credits ledger in dollars, and a round that
        would pass the cap does not start. Prices from OpenAI&apos;s price list
        {status ? `, checked ${status.pricesChecked}` : ''}.
      </p>
    </Panel>
  );
}
