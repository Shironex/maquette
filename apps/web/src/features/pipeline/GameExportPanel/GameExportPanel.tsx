import type { ReactNode } from 'react';
import { Button, HelpMark, SegmentedControl, Select, Toggle } from '@maquette/ui';
import type { GameSettings, GameSettingsChange } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { TEXTURE_CAPS, capLine, useGameExportPanel } from './GameExportPanel.hooks';

export interface GameExportPanelProps {
  settings: GameSettings;
  /** What the models weigh now, newest version of each. */
  onDisk: number;
  busy: boolean;
  onSave: (change: GameSettingsChange) => Promise<boolean>;
  /** Sits under the panel's title: the way to fill these settings from a template. */
  templates?: ReactNode;
}

const FORMATS = [
  { value: 'glb' as const, label: '.glb' },
  { value: 'obj' as const, label: '.obj + .mtl' },
];
const TEXTURES = [
  { value: 'png' as const, label: 'PNG' },
  { value: 'jpg' as const, label: 'JPG' },
  { value: 'keep' as const, label: 'Keep' },
];
const CAP_OPTIONS = TEXTURE_CAPS.map(cap => ({ value: String(cap), label: `${cap} px` }));

const label = 'text-[12px] leading-4 font-medium text-fg-dim';

/** What Game export writes: the format, the textures and their size cap. */
export default function GameExportPanel({
  settings,
  onDisk,
  busy,
  onSave,
  templates,
}: GameExportPanelProps) {
  const { draft, dirty, update, save, saving, reset } = useGameExportPanel(settings, onSave);
  const blocked = busy ? 'Wait for the run to finish' : 'Nothing changed yet';
  return (
    <section
      aria-labelledby="game-export-title"
      className="flex w-[380px] flex-none flex-col gap-3 rounded-card border border-line bg-panel px-4 pt-4 pb-3.5"
    >
      <div>
        <div className="flex items-center gap-1.5">
          <h2 id="game-export-title" className="text-[15px] leading-5 font-semibold">
            Game export
          </h2>
          <HelpMark
            label="About the game export"
            tip="Game export writes one plain model file per asset into web/game/, the way the game's engine reads it. Ship copies them into the model folder."
            side="bottom"
          />
        </div>
        <p className="text-meta mt-0.5 text-fg-mute">What the game&apos;s engine can read</p>
        {templates}
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-auto">
        <div className="flex flex-col gap-1.5">
          <span aria-hidden="true" className={label}>
            Format
          </span>
          <SegmentedControl
            label="Format"
            tone="clay"
            block
            options={FORMATS}
            value={draft.format}
            onChange={format => update({ format })}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <span aria-hidden="true" className={label}>
            Textures
          </span>
          <SegmentedControl
            label="Textures"
            tone="clay"
            block
            options={TEXTURES}
            value={draft.textures}
            onChange={textures => update({ textures })}
          />
        </div>
        <div className="flex flex-col gap-1">
          <Select
            label="Texture size cap"
            size="sm"
            options={CAP_OPTIONS}
            value={String(draft.textureCap)}
            onChange={value => update({ textureCap: Number(value) })}
            disabled={busy}
          />
          <p className="text-meta text-fg-mute">{capLine(settings)}</p>
        </div>
        <div className="flex flex-col gap-1">
          <Toggle
            label="Drop normal and metal maps"
            checked={draft.dropMaps}
            disabled={busy}
            onChange={dropMaps => update({ dropMaps })}
          />
          <p className="text-meta text-fg-mute">
            {draft.dropMaps
              ? 'The export leaves normal and metal-roughness maps out, for a shader that reads neither.'
              : 'Kept: models export with their normal and metal-roughness maps.'}
          </p>
        </div>
        <div className="flex flex-col gap-1.5">
          <span className={label}>LOD levels</span>
          <ul className="m-0 flex list-none flex-col rounded-[7px] border border-line bg-raised p-0 text-meta">
            <li className="flex items-center gap-2 px-2.5 py-1.5">
              <span className="font-mono font-medium">LOD0</span>
              <span className="flex-1 text-fg-mute">full model</span>
              <span className="tabular-nums">100%</span>
            </li>
            <li
              className="flex items-center gap-2 border-t border-line px-2.5 py-1.5 text-fg-mute"
              aria-disabled="true"
            >
              <span className="font-mono">LOD1</span>
              <span className="flex-1">&lt;name&gt;_lod1</span>
              <span>not built yet</span>
            </li>
          </ul>
        </div>
        <div className="flex flex-col gap-1">
          <Toggle
            label="Bake light"
            checked={draft.bake}
            disabled={busy}
            onChange={bake => update({ bake })}
          />
          <p className="text-meta text-fg-mute">
            {draft.bake
              ? 'Bake light runs before the export and bakes a lightmap per model.'
              : 'Off: games light at runtime, so the pipeline skips Bake light.'}
          </p>
        </div>
        <p className="text-meta rounded-[8px] bg-raised px-2.5 py-2 text-fg-dim">
          KTX2 and meshopt are off for game projects. The engine reads plain{' '}
          {draft.format === 'obj' ? '.obj with its .mtl' : 'glTF'}.
        </p>
      </div>
      <div className="flex items-center gap-3 border-t border-line pt-3">
        <div className="flex-1">
          <div className="text-meta text-fg-mute">Est. size on disk now</div>
          <div className="font-mono text-[15px] leading-5 tabular-nums">
            {onDisk > 0 ? formatBytes(onDisk) : 'no models yet'}
          </div>
        </div>
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
          variant="primary"
          loading={saving}
          disabled={!dirty || busy}
          disabledReason={blocked}
          tooltip="Saves these export settings for the next run"
          onClick={() => void save()}
        >
          Apply
        </Button>
      </div>
    </section>
  );
}
