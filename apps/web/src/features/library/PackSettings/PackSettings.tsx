import type { ReactNode } from 'react';
import {
  Button,
  Checkbox,
  PathField,
  SegmentedControl,
  Select,
  TextField,
  Toggle,
} from '@maquette/ui';
import type { PackFormat } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { FORMAT_LABEL, TEXTURE_SIZES } from '@/state/packs';
import { EarlierPacks } from '../RecentPacks';
import BudgetTable from './BudgetTable';
import { usePackSettings } from './PackSettings.hooks';

function Row({ label, note, children }: { label: string; note?: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[124px_minmax(0,1fr)] items-start gap-3 border-b border-line py-2.5 last:border-b-0">
      <span className="pt-1.5 font-semibold">{label}</span>
      <div className="flex min-w-0 flex-col gap-1">
        {children}
        {note ? <span className="text-meta text-fg-mute">{note}</span> : null}
      </div>
    </div>
  );
}

const FORMATS: { value: PackFormat; label: string }[] = [
  { value: 'obj', label: FORMAT_LABEL.obj },
  { value: 'glb', label: FORMAT_LABEL.glb },
  { value: 'both', label: FORMAT_LABEL.both },
];

const TEXTURES = [
  ...TEXTURE_SIZES.map(px => ({ value: String(px), label: `${px} px PNG, base colour` })),
  { value: '0', label: 'As each model has them' },
];

/**
 * The pack's settings, as a preset holds them: the format, size, pivot,
 * materials and textures, a triangle budget per kind, what goes in the zip
 * beside the models and where it is written. Nothing is written until
 * Export and check.
 */
export default function PackSettings() {
  const s = usePackSettings();
  const { settings, plan } = s;
  if (!settings) return <div className="flex-1" />;
  const glb = settings.format !== 'obj';

  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-3">
        <EarlierPacks
          recent={s.recent}
          busy={s.busy}
          onOpen={id => void s.showRecord(id)}
          onResume={id => void s.resume(id)}
        />
        <section aria-label="Pack settings" className="flex flex-col">
          <Row label="Preset" note={s.presetNote}>
            <div className="flex items-center gap-2">
              <Select
                aria-label="Preset"
                options={s.presetOptions}
                value={s.preset}
                onChange={s.pickPreset}
                fieldClassName="flex-1"
              />
              {s.naming ? null : (
                <Button
                  size="sm"
                  icon="plus"
                  tooltip="Keeps these settings under a name for the next pack"
                  onClick={s.startNaming}
                >
                  Save as preset
                </Button>
              )}
            </div>
            {s.naming ? (
              <div className="flex items-center gap-2">
                <TextField
                  aria-label="Preset name"
                  placeholder="Village props for Godot"
                  value={s.presetName}
                  maxLength={60}
                  autoFocus
                  onChange={e => s.setPresetName(e.target.value)}
                  fieldClassName="flex-1"
                />
                <Button size="sm" variant="primary" onClick={s.savePreset}>
                  Save
                </Button>
                <Button size="sm" variant="ghost" onClick={s.stopNaming}>
                  Cancel
                </Button>
              </div>
            ) : null}
          </Row>
          <Row label="Name" note="The folder and the zip are called this.">
            <TextField
              aria-label="Pack name"
              value={settings.name}
              maxLength={60}
              onChange={e => s.change({ name: e.target.value })}
            />
          </Row>
          <Row label="Format">
            <SegmentedControl
              label="Format"
              tone="clay"
              options={FORMATS}
              value={settings.format}
              onChange={format => s.change({ format })}
            />
          </Row>
          <Row label="Up axis" note="Every model is written Y up, the way most engines read it.">
            <span className="pt-1.5">Y up</span>
          </Row>
          <Row label="Units">
            <Toggle
              label={settings.realSize ? 'Metres, real size' : 'Metres, as each model was made'}
              checked={settings.realSize}
              onChange={realSize => s.change({ realSize })}
            />
          </Row>
          <Row label="Pivot">
            <Toggle
              label={
                settings.pivotBottom ? 'Bottom centre, on the floor' : 'As each model was made'
              }
              checked={settings.pivotBottom}
              onChange={pivotBottom => s.change({ pivotBottom })}
            />
          </Row>
          <Row label="Materials">
            <Toggle
              label={
                settings.oneMaterial
                  ? 'Named body (body_2 and on for more)'
                  : 'As each model has them'
              }
              checked={settings.oneMaterial}
              onChange={oneMaterial => s.change({ oneMaterial })}
            />
          </Row>
          <Row label="Texture">
            <Select
              aria-label="Texture"
              options={TEXTURES}
              value={String(settings.texture)}
              onChange={v => s.change({ texture: Number(v) })}
            />
          </Row>
          {settings.format === 'obj' ? (
            <Row label="Rigged models" note={s.mixamoNote}>
              <Toggle
                label="Also write a GLB with the bones"
                checked={settings.riggedGlb}
                onChange={riggedGlb => s.change({ riggedGlb })}
              />
            </Row>
          ) : (
            <Row label="Rigged models" note={s.mixamoNote}>
              <span className="pt-1.5">The GLB keeps the bones</span>
            </Row>
          )}
          {glb ? (
            <Row label="Compression" note="The check opens each file before it is compressed.">
              <Select
                aria-label="Compression"
                options={[
                  { value: 'none', label: 'Plain GLB any importer opens' },
                  { value: 'web', label: "Meshopt and KTX2, the workspace's compress settings" },
                ]}
                value={settings.compression}
                onChange={compression => s.change({ compression })}
              />
            </Row>
          ) : null}
        </section>
        <BudgetTable kinds={plan?.kinds ?? []} settings={settings} onBudget={s.setBudget} />
        <section aria-label="In the pack" className="flex flex-col gap-2">
          <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
            Beside the models
          </h3>
          <div className="flex flex-wrap gap-x-5 gap-y-2">
            <Checkbox
              label="README"
              checked={settings.readme}
              onChange={readme => s.change({ readme })}
            />
            <Checkbox
              label="manifest.json"
              checked={settings.manifest}
              onChange={manifest => s.change({ manifest })}
            />
            <Checkbox
              label="Contact sheet"
              checked={settings.contactSheet}
              onChange={contactSheet => s.change({ contactSheet })}
            />
            <Checkbox
              label="Source refs"
              checked={settings.refs}
              onChange={refs => s.change({ refs })}
            />
          </div>
          <Toggle
            label={`Also write ${settings.name || 'the pack'}.zip next to the folder`}
            checked={settings.zip}
            onChange={zip => s.change({ zip })}
          />
          <PathField
            label="Where"
            value={s.where}
            readOnly
            onBrowse={s.pickFolder}
            browseLabel="Choose"
          />
        </section>
        {s.error ? (
          <p role="alert" className="text-meta text-danger">
            {s.error}
          </p>
        ) : null}
      </div>
      <div className="flex items-center gap-3 border-t border-line px-5 py-3.5">
        <span className="min-w-0 flex-1 text-meta text-fg-mute">
          {plan?.estimateBytes ? `About ${formatBytes(plan.estimateBytes)} · ` : ''}
          Blender runs each model, then opens the pack again to check it · no credits
        </span>
        <Button onClick={s.close}>Cancel</Button>
        <Button
          variant="primary"
          icon="package"
          loading={s.busy}
          disabled={!plan}
          disabledReason={s.error ?? 'Reading the models'}
          onClick={s.start}
        >
          Export and check
        </Button>
      </div>
    </>
  );
}
