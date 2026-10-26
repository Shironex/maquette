import { Icon, cn } from '@maquette/ui';
import { PageFrame } from '../PageFrame';
import { useDevicesBoard } from './DevicesBoard.hooks';

/** The same page at phone, laptop and desktop width, each following the scrubber. */
export default function DevicesBoard() {
  const { src, address, badge, showStops, devices } = useDevicesBoard();
  const frames = devices.map(d => (
    <section
      key={d.id}
      aria-label={d.label}
      className={cn(
        'flex min-h-0 min-w-0 flex-col gap-2',
        d.id === 'phone' ? 'w-[30%] flex-none' : 'flex-1'
      )}
    >
      <header className="flex items-center gap-1.5 text-[12px] whitespace-nowrap">
        <Icon name={d.icon} size={14} className="flex-none text-fg-mute" />
        <span className="font-semibold text-fg">{d.label}</span>
        <span className="font-mono text-fg-mute">{d.size}</span>
        <span className="ml-auto truncate font-mono text-fg-mute">{d.frame}</span>
      </header>
      <PageFrame
        id={d.id}
        src={src}
        address={address}
        badge={badge}
        device={d.id}
        compact
        showStop={showStops}
        label={`The page at ${d.label} width`}
        className="flex-1"
      >
        <p className="m-0 p-4 text-meta text-fg-mute">The page shows here once it runs.</p>
      </PageFrame>
    </section>
  ));
  const [phone, ...wide] = frames;
  // The phone stands tall on the left; the two wide screens stack beside it.
  return (
    <div className="flex min-h-0 flex-1 gap-3">
      {phone}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">{wide}</div>
    </div>
  );
}
