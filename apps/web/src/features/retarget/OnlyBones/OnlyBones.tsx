import { Select } from '@maquette/ui';
import { onlyGroups } from '@/state/retarget';
import { useOnlyBones } from './OnlyBones.hooks';

/**
 * The bones only on the model, a chain to a row. On their own they move with
 * the driven bone above them; a choice here makes them follow another one in
 * the bake (a sword socket that follows the hand, a skirt that follows the
 * hips rather than a thigh).
 */
export default function OnlyBones({ name }: { name: string }) {
  const { sheet, setFollow } = useOnlyBones();
  const plan = sheet?.plan;
  if (!sheet || !plan?.onlyBones.length) return null;
  const { groups, fingers } = onlyGroups(plan.onlyBones);
  const leads = [...new Set(plan.map.flatMap(p => (p.target ? [p.target] : [])))];
  const chosen = (bone: string, saved: string | null) => {
    const here = sheet.follow.find(f => f.bone === bone);
    return here ? here.follows : saved;
  };
  const rows = groups.map(group => {
    const stay = group.carriedBy ? `Follow ${group.carriedBy}, above it` : 'Stay at rest';
    const options = [
      { value: '', label: stay },
      ...leads
        .filter(lead => lead !== group.carriedBy)
        .map(lead => ({ value: lead, label: `Follow ${lead}` })),
    ];
    return (
      <tr key={group.id} className="border-b border-line">
        <td className="py-1 font-mono text-[12px]">{group.label}</td>
        <td className="py-1 pr-2 text-meta text-fg-mute">no source</td>
        <td className="py-1 pr-2">
          {group.free ? (
            <Select
              aria-label={`What ${group.label} follows`}
              size="sm"
              options={options}
              value={chosen(group.bones[0] ?? '', group.follows) ?? ''}
              onChange={lead => setFollow(group.bones, lead || null)}
            />
          ) : (
            <span className="text-meta text-fg-dim">Carries the rig, stays as it is</span>
          )}
        </td>
        <td className="py-1 text-right font-mono text-[11.5px] whitespace-nowrap text-fg-mute">
          {group.bones.length} {group.bones.length === 1 ? 'bone' : 'bones'}
        </td>
      </tr>
    );
  });
  return (
    <section aria-label={`Only on ${name}`} className="flex flex-col gap-1">
      <p className="text-caption tracking-[0.04em] text-fg-mute uppercase">
        Only on {name} · {plan.onlyBones.length}, not in the clips
      </p>
      <table className="w-full border-collapse text-left">
        <tbody>{rows}</tbody>
      </table>
      {fingers ? (
        <p className="text-meta text-fg-mute">
          Fingers · {fingers} {fingers === 1 ? 'bone stays' : 'bones stay'} at rest in the hand.
        </p>
      ) : null}
    </section>
  );
}
