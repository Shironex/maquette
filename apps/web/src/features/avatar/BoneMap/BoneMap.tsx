import { Button, Icon, cn } from '@maquette/ui';
import type { VrmBone } from '@/ipc/client';
import { Panel } from '@/features/common/Panel';
import { SectionLabel } from '@/features/common/SectionLabel';
import { boneLabel } from '@/state/avatar';
import { useBoneMap } from './BoneMap.hooks';

function BoneRow({ bone }: { bone: VrmBone }) {
  return (
    <li className="flex min-w-0 items-center gap-2 py-1 text-meta">
      <Icon
        name={bone.joint ? 'check-circle' : bone.required ? 'x-circle' : 'circle'}
        size={14}
        className={cn(
          'flex-none',
          bone.joint ? 'text-sage' : bone.required ? 'text-danger' : 'text-fg-mute'
        )}
      />
      <span className="w-[136px] flex-none truncate text-fg-dim">{boneLabel(bone.bone)}</span>
      <span
        title={bone.joint ?? undefined}
        className={cn(
          'min-w-0 flex-1 truncate font-mono text-[11.5px]',
          bone.joint ? 'text-fg' : 'text-fg-mute'
        )}
      >
        {bone.joint ?? 'missing'}
      </span>
      {bone.naming === 'override' ? (
        <span className="flex-none rounded-pill border border-line px-1.5 text-[11px] leading-4 text-fg-mute">
          set by hand
        </span>
      ) : null}
    </li>
  );
}

/** Which joint each VRM humanoid bone maps to: the 15 required, then the rest. */
export default function BoneMap() {
  const { readiness, groups: list, missing, toggle } = useBoneMap();
  if (!readiness) return null;
  const groups = list.map(group => {
    const { shown } = group;
    const rows = group.bones.map(b => <BoneRow key={b.bone} bone={b} />);
    return (
      <div key={group.id} className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-2">
          <SectionLabel>
            {group.label} · {group.mapped} of {group.bones.length}
          </SectionLabel>
          {group.id !== 'required' ? (
            <Button
              variant="ghost"
              size="sm"
              icon={shown ? 'caret-up-down' : 'caret-down'}
              tooltip={
                shown
                  ? `Hides the ${group.label.toLowerCase()} bones`
                  : `Lists the ${group.label.toLowerCase()} bones`
              }
              onClick={() => toggle(group.id)}
            >
              {shown ? 'Hide' : 'Show'}
            </Button>
          ) : null}
        </div>
        {shown ? (
          <ul className="m-0 grid list-none grid-cols-1 gap-x-4 p-0 2xl:grid-cols-2">{rows}</ul>
        ) : null}
      </div>
    );
  });
  return (
    <Panel
      id="avatar-bones"
      title="Bone map"
      help={
        missing > 0
          ? `${missing} required ${missing === 1 ? 'bone is' : 'bones are'} missing. Rename the joints, or have an agent map them with set_avatar.`
          : `${readiness.required} of 15 required bones and ${readiness.mapped} of 55 in all.`
      }
    >
      {groups}
    </Panel>
  );
}
