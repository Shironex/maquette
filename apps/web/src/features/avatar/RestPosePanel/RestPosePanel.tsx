import { Button, Icon, InlineBanner, SegmentedControl, cn } from '@maquette/ui';
import type { Elbows } from '@/ipc/client';
import { Panel } from '@/features/common/Panel';
import { SheetZoom } from '../SheetZoom';
import { useRestPosePanel } from './RestPosePanel.hooks';

const ELBOWS: readonly { value: Elbows; label: string; hint: string }[] = [
  { value: 'straighten', label: 'Straighten', hint: 'Levels the lower arms too' },
  { value: 'keep', label: 'Keep', hint: 'Leaves bent elbows bent' },
];

function SheetButton({
  src,
  label,
  onOpen,
}: {
  src: string | null;
  label: string;
  onOpen: () => void;
}) {
  return (
    <figure className="m-0 flex min-w-0 flex-1 flex-col gap-1.5">
      <figcaption className="text-meta font-medium text-fg-dim">{label}</figcaption>
      <button
        type="button"
        onClick={onOpen}
        title={`Opens the ${label.toLowerCase()} sheet up close`}
        className="group relative cursor-zoom-in overflow-hidden rounded-[8px] border border-line bg-studio"
      >
        {src ? (
          <img src={src} alt={`The ${label.toLowerCase()} sheet`} className="block w-full" />
        ) : (
          <span className="block aspect-square" />
        )}
        <span className="absolute right-2 bottom-2 grid size-7 place-items-center rounded-[7px] bg-panel/90 text-fg-dim opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100">
          <Icon name="magnifying-glass" size={15} />
        </span>
      </button>
    </figure>
  );
}

/**
 * The rest pose: its measures, and after a normalise the before and after
 * sheets that a person approves or denies. Nothing is written until then.
 */
export default function RestPosePanel() {
  const p = useRestPosePanel();
  const { view, normalized, state } = p;
  if (!view?.readiness) return null;
  const rows = p.rows.map(r => (
    <tr key={r.id} className="border-t border-line first:border-t-0">
      <th scope="row" className="py-1.5 pr-3 text-left font-normal text-fg-dim">
        {r.label}
      </th>
      <td className="py-1.5 pr-3 text-right font-mono tabular-nums">{r.before}</td>
      {r.after !== null ? (
        <td
          className={cn(
            'py-1.5 text-right font-mono tabular-nums',
            r.fixed ? 'text-sage' : 'text-fg'
          )}
        >
          {r.after}
        </td>
      ) : null}
    </tr>
  ));
  const left = p.findingsAfter.map(f => <li key={f.id}>{f.title}</li>);
  const table = (
    <table className="w-full text-meta">
      <thead>
        <tr className="text-caption tracking-[0.04em] text-fg-mute uppercase">
          <th scope="col" className="pb-1 text-left font-normal">
            Measure
          </th>
          <th scope="col" className="pb-1 text-right font-normal">
            {normalized ? 'Before' : 'Now'}
          </th>
          {normalized ? (
            <th scope="col" className="pb-1 text-right font-normal">
              After
            </th>
          ) : null}
        </tr>
      </thead>
      <tbody>{rows}</tbody>
    </table>
  );
  const elbows = (
    <SegmentedControl
      label="Elbows"
      tone="clay"
      options={ELBOWS}
      value={p.elbows}
      onChange={p.setElbows}
    />
  );
  const help = normalized
    ? `v${normalized.sourceVersion}. ${p.summary}`
    : view.needsNormalize
      ? 'Not VRM’s T-pose yet. A build turns and lifts a copy headless in Blender, then shows the before and after sheets here for your yes.'
      : 'Already VRM’s rest pose: a build writes from the model as it is.';
  return (
    <Panel
      id="avatar-rest-pose"
      title="Rest pose"
      help={help}
      actions={
        normalized && state !== 'pending' ? (
          <span className="flex items-center gap-2 text-meta text-fg-mute">Elbows {elbows}</span>
        ) : null
      }
    >
      {normalized ? (
        <>
          {state === 'pending' ? (
            <section
              aria-label="Rest pose approval"
              className="flex flex-col gap-3 rounded-card border border-amber/40 bg-amber-soft/40 p-3.5"
            >
              <div className="flex items-start gap-3">
                <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-amber-soft text-amber">
                  <Icon name="person-arms-spread" size={17} />
                </span>
                <div className="min-w-0 flex-1">
                  <h3 className="text-emphasis leading-5 font-semibold">
                    Approve the new rest pose?
                  </h3>
                  <p className="mt-0.5 text-meta text-fg-mute">
                    Look at the armpits and whatever hangs by the arms, like a cape or hair, on the
                    after sheet. Approving writes the VRM from this pose; the model’s versions stay
                    as they are.
                  </p>
                </div>
              </div>
              <div className="flex justify-end gap-2">
                <Button
                  variant="secondary"
                  disabled={p.busy}
                  tooltip="Writes nothing; build again with other options"
                  onClick={p.deny}
                >
                  Deny
                </Button>
                <Button
                  variant="primary"
                  icon="check"
                  loading={p.deciding}
                  disabled={p.busy}
                  tooltip={`Approves this pose and writes exports/${view.asset}.vrm`}
                  onClick={p.approve}
                >
                  Approve and write
                </Button>
              </div>
            </section>
          ) : state === 'denied' ? (
            <InlineBanner
              tone="warning"
              title="You turned this pose down"
              detail="Nothing was written. Change the elbows or fix the model, then build again."
            />
          ) : (
            <p className="flex items-center gap-1.5 text-meta text-sage">
              <Icon name="check-circle" size={15} />
              Approved {p.approvedAgo}
            </p>
          )}
          <div className="flex gap-4">
            <SheetButton
              src={p.sheets?.before ?? null}
              label="Before"
              onOpen={() => p.openZoom('before')}
            />
            <SheetButton
              src={p.sheets?.after ?? null}
              label="After"
              onOpen={() => p.openZoom('after')}
            />
          </div>
          <div className="flex flex-wrap items-start gap-6">
            <div className="max-w-[560px] min-w-[280px] flex-1">{table}</div>
            <div className="flex min-w-[240px] flex-1 flex-col gap-2">
              <Button
                variant="secondary"
                size="sm"
                icon="magnifying-glass"
                className="self-start"
                tooltip="The shoulders before and after, side by side and large"
                onClick={() => p.openZoom('shoulders')}
              >
                Compare the shoulders
              </Button>
              {p.findingsAfter.length > 0 ? (
                <ul className="m-0 flex list-none flex-col gap-1 p-0 text-meta text-amber">
                  {left}
                </ul>
              ) : (
                <p className="text-meta text-fg-mute">Nothing left to fix after the normalise.</p>
              )}
            </div>
          </div>
        </>
      ) : (
        <div className="flex flex-wrap items-start gap-6">
          <div className="max-w-[560px] min-w-[280px] flex-1">{table}</div>
          {view.needsNormalize ? (
            <div className="flex flex-col gap-1.5">
              <span className="text-meta font-medium text-fg-dim">Elbows</span>
              {elbows}
            </div>
          ) : null}
        </div>
      )}
      {p.zoom ? (
        <SheetZoom
          before={p.sheets?.before ?? null}
          after={p.sheets?.after ?? null}
          mode={p.zoom}
          onClose={p.closeZoom}
        />
      ) : null}
    </Panel>
  );
}
