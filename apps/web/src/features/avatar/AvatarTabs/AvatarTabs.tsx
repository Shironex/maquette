import { Select, Tabs } from '@maquette/ui';
import type { AvatarTab } from '@/state/use-avatar-store';
import { useAvatarTabs } from './AvatarTabs.hooks';

const TABS = [
  { id: 'export', label: 'Export' },
  { id: 'preview', label: 'Preview' },
] as const;

/** Planned tabs, shown so the person knows they are coming. */
const LATER = [
  { label: 'Springs', why: 'Spring bones for hair and cloth. Not built yet.' },
  { label: 'Expressions', why: 'Blink, mouth and mood expressions. Not built yet.' },
];

/** The avatar's name and version over its tabs: Export, Preview, and the ones still to come. */
export default function AvatarTabs() {
  const tabs = useAvatarTabs();
  if (!tabs) return null;
  const { view, options, setVersion, tab, setTab } = tabs;
  const later = LATER.map(tab => (
    <span
      key={tab.label}
      title={`${tab.label} is planned: ${tab.why}`}
      className="inline-flex h-9 items-center gap-1.5 font-medium text-fg-mute"
    >
      {tab.label}
      <span className="rounded-pill border border-line px-1.5 text-[11px] leading-4">later</span>
    </span>
  ));
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-baseline gap-2.5">
        <h1 className="text-[20px] leading-7 font-semibold">{view.name}</h1>
        {options.length > 1 && view.version !== null ? (
          <Select
            aria-label="Model version"
            size="sm"
            options={options}
            value={String(view.version)}
            onChange={value => setVersion(Number(value))}
            fieldClassName="w-[170px]"
          />
        ) : (
          <span className="font-mono text-[12px] text-fg-mute">
            {view.version === null ? 'no model' : `v${view.version}`}
          </span>
        )}
      </div>
      <div className="flex flex-none items-end gap-4 border-b border-line">
        <Tabs
          tabs={TABS}
          value={tab}
          onChange={id => setTab(id as AvatarTab)}
          className="border-b-0"
        />
        {later}
      </div>
    </div>
  );
}
