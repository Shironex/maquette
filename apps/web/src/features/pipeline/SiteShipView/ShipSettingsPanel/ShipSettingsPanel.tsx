import { Button, HelpMark, Icon, NumberField, SegmentedControl, TextField } from '@maquette/ui';
import type {
  ShipSettings_Serialize as ShipSettings,
  ShipSettingsChange_Deserialize as SettingsChange,
  ShipStyle,
  SiteShipPlan,
  SiteTarget,
  TextureFormat,
} from '@/ipc/client';
import { useOpenSettings } from '@/state/use-settings-target';
import { NEEDS_REPO, targetOf, useFolderDraft } from './ShipSettingsPanel.hooks';

export interface ShipSettingsPanelProps {
  settings: ShipSettings;
  plan: SiteShipPlan;
  /** The site repo, shortened for the home folder. */
  repo: string | null;
  saving: boolean;
  onSettings: (change: SettingsChange) => Promise<boolean>;
  onTarget: (target: SiteTarget) => Promise<boolean>;
}

const STYLES = [
  {
    value: 'scene',
    label: 'Scene',
    hint: 'Maquette builds the scene: models placed by the layout, versioned names, scene data',
  },
  {
    value: 'models',
    label: 'Hand-built site',
    hint: 'One file per model at its own origin, stable names; smaller site files are kept',
  },
] as const satisfies readonly { value: ShipStyle; label: string; hint: string }[];

const FORMATS = [
  { value: 'ktx2', label: 'KTX2', hint: 'GPU textures: the codec column sets ETC1S or UASTC' },
  { value: 'webp', label: 'WebP', hint: 'Small files any browser loads without a transcoder' },
  { value: 'avif', label: 'AVIF', hint: 'Smaller than WebP, slower to encode and decode' },
] as const satisfies readonly { value: TextureFormat; label: string; hint: string }[];

/** How the project ships: the style, the texture format and where it writes. */
export default function ShipSettingsPanel({
  settings,
  plan,
  repo,
  saving,
  onSettings,
  onTarget,
}: ShipSettingsPanelProps) {
  const { draft, change, block } = useFolderDraft(plan.target);
  const noRepo = plan.target.repo ? undefined : NEEDS_REPO;
  const openSettings = useOpenSettings();
  const ktx2 = settings.textures === 'ktx2';
  const checks = plan.checks.map(c => (
    <li key={c.text} className="flex items-start gap-1.5">
      <Icon
        name={c.ok ? 'check-circle' : 'warning-circle'}
        size={14}
        className={c.ok ? 'mt-0.5 flex-none text-sage' : 'mt-0.5 flex-none text-amber'}
      />
      {c.text}
    </li>
  ));
  return (
    <aside className="flex w-[320px] flex-none flex-col gap-3.5">
      <section className="flex flex-col gap-3 rounded-card border border-line bg-panel p-3.5">
        <div className="flex items-center gap-1.5">
          <h2 className="text-[13px] font-semibold">How it ships</h2>
          <HelpMark
            label="About the ship style"
            tip="Scene suits a site that loads Maquette's scene data. Hand-built site suits a page that places each model itself: every model exports at its own origin under a name that stays the same."
            side="bottom"
          />
        </div>
        <SegmentedControl
          label="Ship style"
          tone="clay"
          block
          options={STYLES}
          value={settings.style}
          onChange={style => void onSettings({ style })}
        />
        <SegmentedControl
          label="Texture format"
          tone="clay"
          block
          options={FORMATS}
          value={settings.textures}
          onChange={textures => void onSettings({ textures })}
        />
        <NumberField
          label="Image quality"
          size="sm"
          min={1}
          max={100}
          value={settings.quality}
          disabled={ktx2 || saving}
          hint={
            ktx2
              ? 'KTX2 has no quality setting: pick WebP or AVIF to use it'
              : 'WebP and AVIF quality, 1 to 100'
          }
          onChange={quality => void onSettings({ quality })}
        />
      </section>

      <section className="flex flex-col gap-2.5 rounded-card border border-line bg-panel p-3.5">
        <h2 className="text-[13px] font-semibold">Where it writes</h2>
        <dl className="m-0 grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 text-meta">
          <dt className="text-fg-mute">Site repo</dt>
          <dd className="m-0 truncate font-mono" title={plan.target.repo ?? ''}>
            {repo ?? 'not set'}
          </dd>
        </dl>
        {noRepo ? (
          <Button size="sm" icon="gear-six" onClick={() => openSettings('project')}>
            Pick the site repo
          </Button>
        ) : null}
        <TextField
          label="Models folder"
          mono
          value={draft.models}
          disabled={Boolean(noRepo)}
          hint={noRepo ?? 'Relative to the site, for example public/models'}
          onChange={e => change({ models: e.target.value })}
        />
        <TextField
          label="Posters folder"
          mono
          value={draft.posters}
          placeholder="none"
          disabled={Boolean(noRepo)}
          hint={noRepo ?? 'Empty copies no posters'}
          onChange={e => change({ posters: e.target.value })}
        />
        <TextField
          label="Scene data file"
          mono
          value={draft.data}
          placeholder="none"
          disabled={Boolean(noRepo)}
          hint={noRepo ?? 'Empty writes no scene data, as a hand-built site wants'}
          onChange={e => change({ data: e.target.value })}
        />
        <div className="flex items-center justify-between gap-2">
          <span className="text-meta text-fg-mute">
            {plan.target.configured
              ? 'In maquette.site.json'
              : 'Default folders, no site config yet'}
          </span>
          <Button
            size="sm"
            variant="secondary"
            loading={saving}
            disabled={block !== null}
            disabledReason={block ?? undefined}
            tooltip="Writes the folders into the site's maquette.site.json, keeping its other keys"
            onClick={() => void onTarget(targetOf(draft))}
          >
            Save folders
          </Button>
        </div>
      </section>

      <section className="flex flex-col gap-2 rounded-card border border-line bg-panel p-3.5">
        <h2 className="text-[13px] font-semibold">Checks before Ship</h2>
        <ul className="m-0 flex list-none flex-col gap-1.5 p-0 text-meta">{checks}</ul>
        <p className="text-meta text-fg-dim">
          Ship never commits. Review the change in git and commit it yourself.
        </p>
      </section>
    </aside>
  );
}
