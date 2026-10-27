import { Button, Icon, InlineBanner } from '@maquette/ui';
import { useDeviceVerdict } from './DeviceVerdict.hooks';

const TITLE = 'm-0 text-caption font-medium tracking-[0.04em] text-fg-mute uppercase';

/** The compare board's side: this run against the device's budget and what changed since the ship. */
export default function DeviceVerdict() {
  const { name, banner, profile, changes, openInspector, openPipeline } = useDeviceVerdict();
  const kv = profile.map(p => (
    <div key={p.label} className="contents">
      <dt className="text-fg-mute">{p.label}</dt>
      <dd className="m-0 text-right font-mono text-fg">{p.value}</dd>
    </div>
  ));
  const changed = changes.map(c => (
    <li key={c.title} className="flex gap-2.5 border-b border-line py-2.5 last:border-b-0">
      <Icon name="check" size={14} className="mt-0.5 flex-none text-fg-dim" />
      <span className="flex min-w-0 flex-col">
        <span className="text-[13px] leading-[18px] font-medium text-fg">{c.title}</span>
        <span className="text-[12px] leading-4 text-fg-mute">{c.detail}</span>
      </span>
    </li>
  ));
  return (
    <div className="flex min-h-full flex-col gap-3.5">
      <div className="flex items-center justify-between gap-2">
        <h2 className={TITLE}>{name} verdict</h2>
        <span className="inline-flex h-[22px] items-center gap-[5px] rounded-control bg-raised px-2 text-[11.5px] font-medium text-fg">
          {name} budget
        </span>
      </div>
      {banner ? (
        <InlineBanner tone={banner.tone} title={banner.title} detail={banner.detail} />
      ) : null}
      <dl className="m-0 grid grid-cols-[auto_1fr] gap-x-3.5 gap-y-1.5 text-[12px] leading-4">
        {kv}
      </dl>
      <div className="h-px flex-none bg-line" />
      <section aria-labelledby="page-changes" className="flex flex-col gap-1">
        <h2 id="page-changes" className={TITLE}>
          What changed since the ship
        </h2>
        {changes.length ? (
          <ul className="m-0 list-none p-0">{changed}</ul>
        ) : (
          <p className="m-0 pt-1 text-meta text-fg-mute">
            Nothing to compare yet: the site holds no models, or the same ones.
          </p>
        )}
      </section>
      <div className="h-px flex-none bg-line" />
      <section aria-labelledby="page-poster" className="flex flex-col gap-2">
        <h2 id="page-poster" className={TITLE}>
          Poster fallback
        </h2>
        <p className="m-0 text-meta text-fg-mute">
          The poster is the still a page shows before its 3D starts, and instead of it with no WebGL
          or reduced motion. With poster shows the site that way, through the page bridge.
        </p>
      </section>
      <span className="flex-1" />
      <div className="flex items-center justify-between gap-2">
        <Button
          icon="cube-focus"
          tooltip="Goes to the Inspector, where the scene is checked"
          onClick={openInspector}
        >
          Open Inspector
        </Button>
        <Button
          icon="flow-arrow"
          tooltip="Goes to the Pipeline, where the scene is built and shipped"
          onClick={openPipeline}
        >
          Open Pipeline
        </Button>
      </div>
      <p className="m-0 text-meta text-fg-mute">
        Shipping still asks for your approval in Pipeline.
      </p>
    </div>
  );
}
