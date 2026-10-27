import { Suspense, lazy } from 'react';
import { Button, Icon, IconButton, SegmentedControl, cn } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import type { BoneMatch } from '@/ipc/client';
import { useBoneMapDialog, type MatchFilter } from './BoneMapDialog.hooks';

const MotionStage = lazy(() => import('@/features/viewer/MotionStage'));

const MATCH: Record<BoneMatch, { label: string; className: string; icon: 'check' | 'warning' }> = {
  exact: { label: 'exact', className: 'bg-sage-soft text-sage', icon: 'check' },
  'by-name': { label: 'by name', className: 'bg-sage-soft text-sage', icon: 'check' },
  missing: { label: 'missing', className: 'bg-amber-soft text-amber', icon: 'warning' },
};

const FILTERS: { value: MatchFilter; label: string }[] = [
  { value: 'all', label: 'All bones' },
  { value: 'missing', label: 'Missing' },
];

/**
 * How a library clip would map onto the selected rig, bone by bone, as the
 * rig reader matched it. The workspace has no motion library yet, so there
 * is no clip to retarget: the dialog says so instead of pretending.
 */
export default function BoneMapDialog() {
  const dialog = useBoneMapDialog();
  if (!dialog.open || !dialog.model) return null;
  const { model, counts } = dialog;
  const boneRows = dialog.rows.map(row => {
    const match = MATCH[row.how];
    return (
      <tr
        key={row.bone}
        className={cn('border-b border-line', row.how === 'missing' && 'bg-amber-soft/40')}
      >
        <td className="py-1.5 font-mono text-[12px]">{row.bone}</td>
        <td className="py-1.5 font-mono text-[12px] text-fg-dim">
          {row.joint ?? 'none, stays at rest'}
        </td>
        <td className="py-1.5">
          <span
            className={cn(
              'inline-flex items-center gap-1 rounded-pill px-2 text-[11.5px] leading-5',
              match.className
            )}
          >
            <Icon name={match.icon} size={11} />
            {match.label}
          </span>
        </td>
      </tr>
    );
  });
  return (
    <div
      className="fixed inset-0 z-[60] flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) dialog.close();
      }}
    >
      <div
        ref={dialog.ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="bone-map-title"
        className="flex h-[700px] max-h-full w-[1180px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
      >
        <header className="flex items-start justify-between gap-4 border-b border-line px-5 pt-[18px] pb-3.5">
          <div>
            <h2 id="bone-map-title" className="text-[17px] leading-6 font-semibold">
              Retarget a library clip onto {model.name}
            </h2>
            <p className="text-meta text-fg-dim">
              A library clip is copied into the project and mapped once; the map is kept for this
              rig.
            </p>
          </div>
          <IconButton icon="x" label="Close" onClick={dialog.close} />
        </header>
        <div className="flex min-h-0 flex-1">
          <aside className="flex w-[240px] flex-none flex-col gap-3 border-r border-line p-4">
            <p className="text-caption tracking-[0.04em] text-fg-mute uppercase">Library</p>
            <div className="flex flex-col items-start gap-2 rounded-card border border-dashed border-line-strong p-3.5">
              <Icon name="books" size={18} className="text-fg-mute" />
              <p className="text-[13px] leading-[18px] font-semibold">No motion library yet</p>
              <p className="text-meta text-fg-mute">
                This workspace has no library clips to retarget. Until it does, add clips from a
                .glb in the Inbox or bake them with a Blender script.
              </p>
            </div>
          </aside>
          <section aria-label="Bone map" className="flex min-w-0 flex-1 flex-col gap-3 p-4">
            <div className="flex items-center justify-between gap-3">
              <p className="text-caption tracking-[0.04em] text-fg-mute uppercase">Bone map</p>
              <span className="font-mono text-[11.5px] text-fg-mute">
                {counts.matched} matched · {counts.missing} missing
              </span>
            </div>
            <SegmentedControl
              label="Bones to show"
              options={FILTERS}
              value={dialog.filter}
              onChange={dialog.setFilter}
              className="self-start"
            />
            <div className="min-h-0 flex-1 overflow-y-auto">
              <table className="w-full border-collapse text-left">
                <thead className="text-caption tracking-[0.04em] text-fg-mute uppercase">
                  <tr className="border-b border-line">
                    <th className="py-2 font-normal">Library bone</th>
                    <th className="py-2 font-normal">{model.name} bone</th>
                    <th className="py-2 font-normal">Match</th>
                  </tr>
                </thead>
                <tbody>{boneRows}</tbody>
              </table>
            </div>
            <p className="text-meta text-fg-mute">
              Showing {dialog.rows.length} of {model.rig?.map.length ?? 0}. Missing bones keep their
              rest pose in a library clip.
            </p>
          </section>
          <aside
            aria-label={`On ${model.name}`}
            className="flex w-[320px] flex-none flex-col gap-3 border-l border-line p-4"
          >
            <p className="text-caption tracking-[0.04em] text-fg-mute uppercase">On {model.name}</p>
            <div className="relative h-[300px] overflow-hidden rounded-card border border-line bg-studio">
              {dialog.url ? (
                <CanvasBoundary resetKey={dialog.url}>
                  <Suspense fallback={null}>
                    <MotionStage
                      className="absolute! inset-0"
                      label={`${model.name} rig at rest`}
                      url={dialog.url}
                      clip={null}
                      time={0}
                      skeleton
                      trail={false}
                      turntable={false}
                      poses={null}
                      outline={[]}
                    />
                  </Suspense>
                </CanvasBoundary>
              ) : null}
            </div>
            <p className="text-meta text-fg-mute">
              The rig at rest with its bones. A library clip would play on it here, side by side
              with its own skeleton, once there is one to map.
            </p>
          </aside>
        </div>
        <footer className="flex flex-wrap items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button onClick={dialog.close}>Cancel</Button>
          <Button
            variant="primary"
            icon="arrows-left-right"
            disabled
            disabledReason="There is no library clip to retarget yet"
            tooltip="Maps the library clip onto this rig"
          >
            Retarget
          </Button>
        </footer>
      </div>
    </div>
  );
}
