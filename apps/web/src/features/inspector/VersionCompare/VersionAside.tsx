import { cn } from '@maquette/ui';
import { formatAgo } from '@/lib/format';
import type { SidePick } from '@/state/inspector';
import { PROVIDER_LABEL } from '@/state/providers';
import { jobForVersion } from '@/state/version-compare';
import { SectionTitle } from '../SectionTitle';
import { useNowOn, type VersionCompareModel } from './VersionCompare.hooks';

const ROW =
  'flex w-full items-center gap-2 rounded-control border border-line px-2.5 py-1.5 text-left';

/** The side panel of two versions: the versions, what keeping does, where to judge them. */
export function VersionAside({ v, pick }: { v: VersionCompareModel; pick: SidePick }) {
  const now = useNowOn(pick.comparison.id);
  const asset = v.asset;
  const versions = [...(asset?.versions ?? [])].sort((a, b) => b.version - a.version);
  const rows = versions.map(ver => {
    const job = asset ? jobForVersion(v.jobs, asset.id, ver.version) : null;
    const side = pick.comparison.sides.find(s => s.members[0]?.model.endsWith(`/v${ver.version}`));
    const tag =
      ver.version === v.rightFacts.version
        ? 'Right'
        : ver.version === v.leftFacts.version
          ? 'Left'
          : ver.version === v.current
            ? 'current'
            : null;
    const onSide = tag === 'Right' || tag === 'Left';
    const made = job ? PROVIDER_LABEL[job.provider] : 'In Maquette';
    return (
      <li key={ver.version}>
        <button
          type="button"
          disabled={onSide || !side}
          aria-label={`Compare v${ver.version} on the left`}
          onClick={() => side && v.pickLeft(side.id)}
          className={cn(
            ROW,
            onSide ? 'border-clay-line' : 'cursor-pointer hover:border-line-strong'
          )}
        >
          <span className="w-6 flex-none font-mono text-fg">v{ver.version}</span>
          <span className="min-w-0 flex-1 truncate text-meta text-fg-mute">
            {made} · {formatAgo(Date.parse(ver.createdAt), now)}
          </span>
          {tag ? <span className="flex-none text-meta text-fg-dim">{tag}</span> : null}
        </button>
      </li>
    );
  });
  const stops = v.atOptions.map(stop => (
    <AtButton
      key={stop.index}
      name={stop.name}
      note={stop.note}
      on={v.stopIndex === stop.index}
      onClick={() => v.goStop(stop.index)}
    />
  ));
  return (
    <aside
      aria-label="Versions"
      className="flex w-[340px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel px-5 py-[18px]"
    >
      <div>
        <h2 className="text-panel-title">{asset?.name ?? pick.comparison.label}</h2>
        <p className="text-meta text-fg-mute">
          {versions.length} {versions.length === 1 ? 'version' : 'versions'}
        </p>
      </div>
      <section className="flex flex-col gap-2">
        <SectionTitle>Versions</SectionTitle>
        <ul className="flex flex-col gap-1.5">{rows}</ul>
      </section>
      <section className="flex flex-col gap-1.5">
        <SectionTitle>What keeping does</SectionTitle>
        <p className="text-meta text-fg-mute">
          The kept version becomes current on the Board and in the next run. The other stays in
          versions, nothing is deleted and no credits come back.
        </p>
      </section>
      <section className="flex flex-col gap-2">
        <SectionTitle>Compare at</SectionTitle>
        <ul className="flex flex-col gap-1.5">
          {stops}
          <AtButton
            name="Turntable, close up"
            note="free orbit"
            on={v.stopIndex === null}
            onClick={v.goClose}
          />
        </ul>
      </section>
    </aside>
  );
}

/** One place to judge the two from. */
function AtButton(props: { name: string; note: string; on: boolean; onClick: () => void }) {
  return (
    <li>
      <button
        type="button"
        aria-pressed={props.on}
        onClick={props.onClick}
        className={cn(
          ROW,
          'cursor-pointer flex-col items-start gap-0',
          props.on ? 'border-clay-line' : 'hover:border-line-strong'
        )}
      >
        <span className="text-fg">{props.name}</span>
        <span className="text-meta text-fg-mute">{props.note}</span>
      </button>
    </li>
  );
}
