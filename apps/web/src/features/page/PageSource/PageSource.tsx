import { Button, Icon, InlineBanner, cn } from '@maquette/ui';
import { usePageSource } from './PageSource.hooks';

interface SourceRowProps {
  on: boolean;
  name: string;
  line: string;
  onPick: () => void;
  badge?: 'live' | 'mock';
}

function SourceRow({ on, name, line, onPick, badge }: SourceRowProps) {
  return (
    <label
      className={cn(
        'flex w-full cursor-pointer items-center gap-2.5 rounded-card border px-2.5 py-2 text-left',
        on ? 'border-clay-line bg-clay-soft' : 'border-transparent hover:bg-hover'
      )}
    >
      <input type="radio" name="page-source" checked={on} onChange={onPick} className="sr-only" />
      <span
        aria-hidden="true"
        className={cn(
          'grid size-4 flex-none place-items-center rounded-full border-[1.5px]',
          on ? 'border-clay-text bg-clay text-clay-ink' : 'border-line-strong'
        )}
      >
        {on ? <Icon name="check" size={10} /> : null}
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="text-[13px] leading-[17px] font-medium text-fg">{name}</span>
        <span className="truncate font-mono text-[11.5px] leading-4 text-fg-mute">{line}</span>
      </span>
      {badge ? (
        <span className="inline-flex h-[22px] flex-none items-center gap-[5px] rounded-control bg-[rgba(28,27,25,0.78)] px-2 text-[11.5px] font-medium text-fg">
          {badge === 'live' ? (
            <span aria-hidden="true" className="size-1.5 rounded-full bg-sage" />
          ) : (
            <Icon name="file-text" size={12} className="text-fg-dim" />
          )}
          {badge === 'live' ? 'Live' : 'Generated mock'}
        </span>
      ) : null}
    </label>
  );
}

/** Which page the view shows: the site's own dev page or an HTML page concept. */
export default function PageSource() {
  const { source, siteLine, live, conceptLine, hasConcept, showSite, showConcept, changeConcept } =
    usePageSource();
  return (
    <section aria-labelledby="page-source" className="flex flex-col gap-2.5">
      <div className="flex items-center justify-between gap-2">
        <h2
          id="page-source"
          className="m-0 text-caption font-medium tracking-[0.04em] text-fg-mute uppercase"
        >
          Source
        </h2>
        {hasConcept ? (
          <Button
            size="sm"
            variant="ghost"
            icon="folder-open"
            tooltip="Choose a different HTML page concept"
            onClick={changeConcept}
          >
            Other concept
          </Button>
        ) : null}
      </div>
      <div role="radiogroup" aria-labelledby="page-source" className="flex flex-col gap-0.5">
        <SourceRow
          on={source === 'site'}
          name="Site dev page"
          line={siteLine}
          onPick={showSite}
          badge={source === 'site' && live ? 'live' : undefined}
        />
        <SourceRow
          on={source === 'concept'}
          name="HTML concept"
          line={conceptLine}
          onPick={showConcept}
          badge={source === 'concept' ? 'mock' : undefined}
        />
      </div>
      <InlineBanner
        tone="info"
        title="What is live here"
        detail="The site dev page is the site's own dev server, so the 3D scene is the models the site repo holds, the last ship, and the frame time comes from the page itself. This run's numbers come from its build until Ship copies it to the site. With a generated mock the address bar says Mock, it shows as it is with no site needed, and only the scene numbers count."
      />
    </section>
  );
}
