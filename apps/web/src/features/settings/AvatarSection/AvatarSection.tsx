import { Checkbox, SegmentedControl, Select, TextField, cn } from '@maquette/ui';
import type { AvatarDefaults, WorkspaceConfig } from '@/ipc/client';
import { Panel } from '@/features/common/Panel';
import {
  COMMERCIAL,
  CREDIT,
  MATERIALS,
  MODIFICATION,
  PERMISSIONS,
  USAGE_FLAGS,
} from '@/state/avatar';
import { ACTIVE } from '../sections';

export interface AvatarSectionProps {
  active: boolean;
  config: WorkspaceConfig;
  onChange: (config: WorkspaceConfig) => void;
}

const orNull = (v: string) => (v.trim() ? v : null);

/** Settings > Avatar defaults: what every new VRM avatar's meta starts from. */
export default function AvatarSection({ active, config, onChange }: AvatarSectionProps) {
  const d = config.avatar ?? {};
  const set = (change: Partial<AvatarDefaults>) =>
    onChange({ ...config, avatar: { ...d, ...change } });
  const flags = USAGE_FLAGS.map(flag => (
    <Checkbox
      key={flag.key}
      label={flag.label}
      checked={d[flag.key] ?? false}
      onChange={on => set({ [flag.key]: on })}
    />
  ));
  return (
    <Panel
      id="settings-avatar"
      title="Avatar defaults"
      help="Who makes this workspace's VRM avatars and the licence they go out under. Each new avatar starts from these."
      className={cn('gap-3 px-4 py-3.5', active && ACTIVE)}
    >
      <div className="grid grid-cols-2 gap-2.5">
        <TextField
          label="Author"
          placeholder="Your name or studio"
          value={d.author ?? ''}
          onChange={e => set({ author: e.target.value })}
          fieldClassName="col-span-2"
        />
        <TextField
          label="Contact"
          placeholder="Optional"
          value={d.contactInformation ?? ''}
          onChange={e => set({ contactInformation: orNull(e.target.value) })}
        />
        <TextField
          label="Copyright"
          placeholder="Optional"
          value={d.copyrightInformation ?? ''}
          onChange={e => set({ copyrightInformation: orNull(e.target.value) })}
        />
        <Select
          label="Who may play it"
          options={PERMISSIONS}
          value={d.avatarPermission ?? 'onlyAuthor'}
          onChange={v => set({ avatarPermission: v })}
        />
        <Select
          label="Commercial use"
          options={COMMERCIAL}
          value={d.commercialUsage ?? 'personalNonProfit'}
          onChange={v => set({ commercialUsage: v })}
        />
        <Select
          label="Credit"
          options={CREDIT}
          value={d.creditNotation ?? 'required'}
          onChange={v => set({ creditNotation: v })}
        />
        <Select
          label="Changes"
          options={MODIFICATION}
          value={d.modification ?? 'prohibited'}
          onChange={v => set({ modification: v })}
        />
      </div>
      <div className="grid grid-cols-2 gap-x-3 gap-y-2">{flags}</div>
      <TextField
        label="Other licence URL"
        placeholder="Optional, on top of the VRM licence"
        mono
        value={d.otherLicenseUrl ?? ''}
        onChange={e => set({ otherLicenseUrl: orNull(e.target.value) })}
      />
      <div className="flex flex-col gap-1.5">
        <span className="text-meta font-medium text-fg-dim">Material</span>
        <SegmentedControl
          label="Material"
          tone="clay"
          options={MATERIALS.map(m => ({ value: m.value, label: m.label, hint: m.hint }))}
          value={d.material ?? 'mtoon-soft'}
          onChange={material => set({ material })}
        />
      </div>
      <p className="text-meta text-fg-mute">
        The licence starts at VRM 1.0&apos;s defaults, the strictest. An avatar can change its own
        in the Avatar view.
      </p>
    </Panel>
  );
}
