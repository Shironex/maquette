import { cn } from '@maquette/ui';
import { usePageReadouts } from './PageReadouts.hooks';

/** The page as it runs: frame time from the page, and the scene the site holds now. */
export default function PageReadouts() {
  const { cards, help, onSite } = usePageReadouts();
  const tiles = cards.map(c => (
    <div
      key={c.label}
      className="flex min-w-0 flex-col gap-0.5 rounded-card border border-line bg-raised px-2.5 py-2"
    >
      <span className="text-[11.5px] leading-[15px] text-fg-mute">{c.label}</span>
      <span
        className={cn(
          'truncate font-mono text-[15px] leading-5 font-medium',
          c.over ? 'text-danger' : 'text-fg'
        )}
      >
        {c.value} <small className="text-[11.5px] text-fg-mute">{c.of}</small>
      </span>
    </div>
  ));
  return (
    <section aria-labelledby="page-readouts" className="flex flex-col gap-2.5">
      <div className="flex items-center justify-between gap-2">
        <h2
          id="page-readouts"
          className="m-0 text-caption font-medium tracking-[0.04em] text-fg-mute uppercase"
        >
          Readouts
        </h2>
        <span className="inline-flex h-[22px] items-center gap-[5px] rounded-control bg-raised px-2 text-[11.5px] font-medium text-fg">
          {onSite ? 'Current ship' : 'Nothing shipped yet'}
        </span>
      </div>
      <div className="grid grid-cols-2 gap-2">{tiles}</div>
      <p className="m-0 text-meta text-fg-mute">{help}</p>
    </section>
  );
}
