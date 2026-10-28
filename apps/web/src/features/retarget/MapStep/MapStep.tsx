import { Checkbox, Icon, SegmentedControl, Select, cn } from '@maquette/ui';
import type { PairMatch } from '@/ipc/client';
import { needsLook } from '@/state/retarget';
import { useMapStep, type MapFilter } from './MapStep.hooks';

const MATCH: Record<PairMatch, { label: string; className: string }> = {
  saved: { label: 'saved map', className: 'bg-sage-soft text-sage' },
  exact: { label: 'exact', className: 'bg-sage-soft text-sage' },
  'by-name': { label: 'by name', className: 'bg-sage-soft text-sage' },
  picked: { label: 'picked', className: 'bg-clay-soft text-fg' },
  missing: { label: 'unmapped, stays at rest', className: 'bg-amber-soft text-amber' },
};

const FILTERS: { value: MapFilter; label: string }[] = [
  { value: 'all', label: 'All bones' },
  { value: 'look', label: 'Needs a look' },
];

/** Step two: the clip's skeleton onto the model's rig, bone by bone. Never blocks. */
export default function MapStep() {
  const step = useMapStep();
  if (!step.sheet) return null;
  const { plan, counts } = step;
  const name = plan?.name ?? 'Model';
  const options = [
    { value: '', label: 'None, stays at rest' },
    ...step.joints.map(j => ({ value: j, label: j })),
  ];
  const rows = step.rows.map(row => {
    const match = MATCH[row.how];
    return (
      <tr
        key={row.bone}
        className={cn('border-b border-line', needsLook(row) && 'bg-amber-soft/40')}
      >
        <td className="py-1 font-mono text-[12px]">{row.bone}</td>
        <td className="py-1 font-mono text-[12px] text-fg-dim">
          {row.source ?? 'not in the clip'}
        </td>
        <td className="py-1 pr-2">
          <Select
            aria-label={`${name} bone for ${row.bone}`}
            size="sm"
            options={options}
            value={row.target ?? ''}
            onChange={target => step.pick(row.bone, target)}
          />
        </td>
        <td className="py-1">
          <span
            className={cn(
              'inline-flex items-center gap-1 rounded-pill px-2 text-[11.5px] leading-5',
              match.className
            )}
          >
            <Icon name={row.how === 'missing' ? 'warning' : 'check'} size={11} />
            {match.label}
          </span>
        </td>
      </tr>
    );
  });
  const only = plan?.onlyOnModel ?? [];
  return (
    <>
      <section aria-label="Bone map" className="flex min-w-0 flex-1 flex-col gap-3 p-4">
        <div className="flex items-center justify-between gap-3">
          <p className="text-meta text-fg-dim">
            {step.source}, {plan?.sourceBones ?? 0} bones onto {name}, {plan?.joints ?? 0} bones
          </p>
          <span className="font-mono text-[11.5px] text-fg-mute">
            {counts.matched} matched · {counts.look} to look at · {counts.picked} picked
          </span>
        </div>
        <SegmentedControl
          label="Bones to show"
          options={FILTERS}
          value={step.filter}
          onChange={step.setFilter}
          className="self-start"
        />
        <div className="min-h-0 flex-1 overflow-y-auto">
          <table className="w-full border-collapse text-left">
            <thead className="text-caption tracking-[0.04em] text-fg-mute uppercase">
              <tr className="border-b border-line">
                <th className="py-2 font-normal">Humanoid bone</th>
                <th className="py-2 font-normal">Clip bone</th>
                <th className="py-2 font-normal">{name} bone</th>
                <th className="py-2 font-normal">Match</th>
              </tr>
            </thead>
            <tbody>{rows}</tbody>
          </table>
        </div>
      </section>
      <aside className="flex w-[300px] flex-none flex-col gap-3 overflow-y-auto border-l border-line p-4">
        <p className="text-caption tracking-[0.04em] text-fg-mute uppercase">How it bakes</p>
        <p className="text-meta text-fg-dim">
          Before baking, {name}&apos;s limbs are turned to the clip&apos;s rest pose so both rigs
          start matched. Each frame then takes the world rotation of every clip bone against its
          rest and applies that change to the mapped bone. The hips move scaled by the hips height.
        </p>
        <p className="text-meta text-fg-dim">
          {only.length
            ? `Only on ${name} · ${only.length}: ${only.slice(0, 4).join(', ')}${only.length > 4 ? ' and more' : ''}. They follow their parents.`
            : `Every ${name} bone has a humanoid bone.`}
        </p>
        <p className="text-meta text-fg-mute">Fingers and unmapped bones stay at rest.</p>
        <Checkbox
          checked={step.sheet.saveMap}
          onChange={step.setSaveMap}
          label={`Save this map for ${name} and reuse it`}
        />
      </aside>
    </>
  );
}
