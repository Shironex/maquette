import { Button, ProgressBar, Tabs } from '@maquette/ui';
import type { PackRecord } from '@/ipc/client';
import NeedsYouCard from './NeedsYouCard';
import PackCheckCard from './PackCheckCard';
import { usePackCheck } from './PackCheck.hooks';

export interface PackCheckProps {
  record: PackRecord;
}

/**
 * A pack while Blender writes and checks it: a card per model with what
 * the check found, the models that need you with their answers, the
 * README the pack will carry, and the zip, which waits until every model
 * passes or you say keep it.
 */
export default function PackCheck({ record }: PackCheckProps) {
  const c = usePackCheck(record);
  const waiting = record.state === 'waiting';
  const cards = c.shown.map(item => (
    <li key={item.model.key}>
      <PackCheckCard item={item} format={record.settings.format} />
    </li>
  ));
  const needCards = c.needs.map(item => (
    <NeedsYouCard key={item.model.key} item={item} busy={c.busy} onDecide={c.decide} />
  ));
  let footer;
  if (record.state === 'stopped' || record.state === 'failed') {
    footer = (
      <>
        <span className="min-w-0 flex-1 text-meta text-fg-dim">
          {record.message ?? 'Stopped before every model ran.'}
        </span>
        {record.state === 'stopped' ? (
          <Button variant="primary" icon="play" loading={c.busy} onClick={c.resume}>
            Go on from {c.stoppedAt}
          </Button>
        ) : null}
      </>
    );
  } else if (record.state === 'zipping') {
    footer = (
      <span className="flex-1 text-meta text-fg-dim">
        Writing the README, manifest, contact sheet and zip
      </span>
    );
  } else {
    footer = (
      <>
        <span className="min-w-0 flex-1 text-meta text-fg-mute">
          {record.message ?? 'The zip waits until every model passes or you say keep it.'}
        </span>
        <Button
          icon="stop"
          disabled={record.state !== 'running' || c.busy || c.stopping}
          disabledReason={c.stopping ? 'Stops once this model is done' : 'Every model has run'}
          tooltip="Stops once the model Blender is on now is done; Go on takes the next one"
          onClick={() => void c.stop()}
        >
          {c.stopping ? 'Stopping' : 'Stop'}
        </Button>
        <Button
          disabled={!waiting || c.passing === 0 || c.busy}
          disabledReason={waiting ? 'No model passes yet' : 'Wait for the models still running'}
          tooltip="Leaves the models that need you out of the pack"
          onClick={() => void c.zip('passing')}
        >
          Zip {c.passing} that pass
        </Button>
        <Button
          variant="primary"
          icon="package"
          disabled={!waiting || c.zipAll === 0 || c.busy}
          disabledReason={waiting ? 'No model has files yet' : 'Wait for the models still running'}
          tooltip="Zips every model with files, the warnings as they are; a failed one has none"
          onClick={() => void c.zip('all')}
        >
          Zip all {c.zipAll}
        </Button>
      </>
    );
  }

  return (
    <>
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-2.5">
        <Tabs tabs={c.tabs} value={c.filter} onChange={c.setFilter} />
        <span className="flex-1" />
        <span
          className="font-mono text-meta text-fg-dim"
          title="A rough guess from the models done so far. A model over budget takes far longer than one left as it is."
        >
          {c.progress}
        </span>
        <ProgressBar value={c.percent} label="Models checked" className="w-[140px]" />
      </div>
      <div className="flex min-h-0 flex-1">
        <section
          aria-label="Models in the pack"
          className="min-w-0 flex-1 overflow-y-auto px-5 py-4"
        >
          {c.shown.length ? (
            <ul className="grid list-none grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-3">
              {cards}
            </ul>
          ) : (
            <p className="py-10 text-center text-fg-mute">No model here.</p>
          )}
        </section>
        <aside
          aria-label="Needs you"
          className="flex w-[300px] flex-none flex-col gap-3 overflow-y-auto border-l border-line px-4 py-4"
        >
          <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
            Needs you{c.needs.length ? ` · ${c.needs.length}` : ''}
          </h3>
          {c.needs.length ? (
            needCards
          ) : (
            <p className="text-meta text-fg-mute">
              Nothing yet. A model that fails or looks off waits here.
            </p>
          )}
          {record.settings.readme ? (
            <>
              <h3 className="pt-2 text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
                README.txt
              </h3>
              <pre className="overflow-x-auto rounded-card border border-line bg-app p-3 font-mono text-[11px] leading-[1.5] whitespace-pre-wrap text-fg-dim">
                {c.readme}
              </pre>
            </>
          ) : null}
          <p className="text-meta text-fg-mute">
            Agents can ask for a pack too: export_pack takes model ids and a preset, and reports
            these same checks.
          </p>
        </aside>
      </div>
      <div className="flex items-center gap-2 border-t border-line px-5 py-3.5">{footer}</div>
    </>
  );
}
