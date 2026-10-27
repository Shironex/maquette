import type { ReactNode } from 'react';
import {
  Button,
  IconButton,
  NumberField,
  SegmentedControl,
  Select,
  TextField,
  type SegmentedOption,
  type SelectOption,
} from '@maquette/ui';
import type { FactorMode, InspectModel } from '@/ipc/client';
import { ALL_MATERIALS, MAX_EMISSIVE, roughnessMax } from '@/state/materials';
import { SectionTitle } from '../SectionTitle';
import { useHexText, useMaterialPanel } from './MaterialPanel.hooks';

type Roughness = 'keep' | FactorMode;

const ROUGHNESS: SegmentedOption<Roughness>[] = [
  { value: 'keep', label: 'Keep' },
  { value: 'override', label: 'Set', hint: 'Replaces the metal and roughness map' },
  { value: 'multiply', label: 'Multiply', hint: 'Scales the roughness and keeps the map' },
];
const KEEP_SET: SegmentedOption<'keep' | 'set'>[] = [
  { value: 'keep', label: 'Keep' },
  { value: 'set', label: 'Set' },
];
const METAL: SegmentedOption<'keep' | 'set'>[] = [
  KEEP_SET[0]!,
  { value: 'set', label: 'Set', hint: 'Replaces the metal and roughness map' },
];
/** Where a value starts when its field turns on. */
const START = { override: 0.8, multiply: 1, metalness: 0, emissive: 1 } as const;
const two = (n: number) => n.toFixed(2);

export interface MaterialPanelProps {
  project: string;
  model: InspectModel;
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex min-h-7 items-center justify-between gap-2">
      <span className="text-fg-dim">{label}</span>
      <div className="flex items-center gap-1.5">{children}</div>
    </div>
  );
}

function Tint({
  value,
  onChange,
}: {
  value: string | null;
  onChange: (hex: string | null) => void;
}) {
  const hex = useHexText(value, onChange);
  return (
    <>
      <input
        type="color"
        aria-label="Tint color"
        value={value ?? '#ffffff'}
        onChange={e => onChange(e.target.value)}
        className="size-7 flex-none cursor-pointer rounded-control border border-line bg-app p-0.5"
      />
      <TextField
        aria-label="Tint hex"
        mono
        placeholder="none"
        value={hex.text}
        onChange={e => hex.setText(e.target.value)}
        onBlur={hex.commit}
        onKeyDown={e => {
          if (e.key === 'Enter') hex.commit();
        }}
        className="h-7"
        fieldClassName="w-[92px]"
      />
      <IconButton
        icon="x"
        size="sm"
        label="Clear the tint"
        disabled={value === null}
        disabledReason="No tint set"
        onClick={() => onChange(null)}
      />
    </>
  );
}

/**
 * Per model material tweaks: tint, roughness, metalness and emissive
 * strength for every material or one by name. Edits preview at once and
 * wait for Apply; Compress writes them into the shipped model.
 */
export default function MaterialPanel({ project, model }: MaterialPanelProps) {
  const p = useMaterialPanel(project, model);
  if (!p.asset) {
    return (
      <div className="flex flex-col gap-2.5">
        <SectionTitle>Material</SectionTitle>
        <p className="text-fg-mute">
          Tweaks belong to a project asset, and no asset makes this file.
        </p>
      </div>
    );
  }
  const targets: SelectOption<string>[] = [
    { value: ALL_MATERIALS, label: 'All materials' },
    ...model.materialNames.map(name => ({ value: name, label: name })),
  ];
  const { tweak, edit } = p;
  const rough = tweak.roughness;
  return (
    <div className="flex flex-col gap-2.5">
      <SectionTitle
        aside={
          p.dirty ? (
            <span role="status" className="text-clay-text">
              Not applied yet
            </span>
          ) : null
        }
      >
        Material
      </SectionTitle>
      <Row label="Applies to">
        <Select
          aria-label="Applies to"
          title={p.target === ALL_MATERIALS ? undefined : p.target}
          size="sm"
          inline
          options={targets}
          value={p.target}
          onChange={p.setTarget}
          fieldClassName="w-[170px]"
        />
      </Row>
      <Row label="Tint">
        <Tint value={tweak.tint} onChange={tint => edit({ tint })} />
      </Row>
      <Row label="Roughness">
        {rough ? (
          <NumberField
            aria-label="Roughness value"
            size="sm"
            value={rough.value ?? 0}
            min={0}
            max={roughnessMax(rough.mode)}
            format={two}
            onChange={value => edit({ roughness: { ...rough, value } })}
            fieldClassName="w-[52px]"
          />
        ) : null}
        <SegmentedControl
          label="Roughness"
          tone="clay"
          options={ROUGHNESS}
          value={rough?.mode ?? 'keep'}
          onChange={mode =>
            edit({ roughness: mode === 'keep' ? null : { mode, value: START[mode] } })
          }
        />
      </Row>
      <Row label="Metalness">
        {tweak.metalness !== null ? (
          <NumberField
            aria-label="Metalness value"
            size="sm"
            value={tweak.metalness}
            min={0}
            max={1}
            format={two}
            onChange={metalness => edit({ metalness })}
            fieldClassName="w-[52px]"
          />
        ) : null}
        <SegmentedControl
          label="Metalness"
          tone="clay"
          options={METAL}
          value={tweak.metalness === null ? 'keep' : 'set'}
          onChange={m => edit({ metalness: m === 'keep' ? null : START.metalness })}
        />
      </Row>
      <Row label="Emissive">
        {tweak.emissiveStrength !== null ? (
          <NumberField
            aria-label="Emissive strength"
            size="sm"
            value={tweak.emissiveStrength}
            min={0}
            max={MAX_EMISSIVE}
            format={n => n.toFixed(1)}
            onChange={emissiveStrength => edit({ emissiveStrength })}
            fieldClassName="w-[52px]"
          />
        ) : null}
        <SegmentedControl
          label="Emissive"
          tone="clay"
          options={KEEP_SET}
          value={tweak.emissiveStrength === null ? 'keep' : 'set'}
          onChange={m => edit({ emissiveStrength: m === 'keep' ? null : START.emissive })}
        />
      </Row>
      <p className="rounded-[8px] bg-raised px-2.5 py-2 text-meta text-fg-mute">
        {p.dropsMap
          ? 'Setting roughness or metalness replaces the metal and roughness map, which holds both.'
          : 'Multiply keeps the maps. The source model never changes; Compress writes the shipped copy.'}
      </p>
      <div className="flex items-center justify-end gap-2">
        <Button
          size="sm"
          disabled={!p.canReset || p.saving}
          disabledReason={p.saving ? 'Saving the tweaks' : 'No tweaks to drop'}
          tooltip="Drops the saved tweaks so the model looks as it came"
          onClick={p.onReset}
        >
          {p.resetLabel}
        </Button>
        <Button
          size="sm"
          variant="primary"
          disabled={!p.dirty || p.saving || p.applyBlocked !== null}
          disabledReason={p.applyBlocked ?? 'Nothing changed yet'}
          tooltip="Saves the tweaks; Compress applies them to the shipped model on the next run"
          loading={p.saving}
          onClick={p.onApply}
        >
          Apply tweaks
        </Button>
      </div>
    </div>
  );
}
