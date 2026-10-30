import { Button, Icon, IconButton, cn } from '@maquette/ui';
import type { VrmBone } from '@/ipc/client';
import { Panel } from '@/features/common/Panel';
import { SectionLabel } from '@/features/common/SectionLabel';
import { boneLabel, boneSource } from '@/state/avatar';
import { BonePicker } from './BonePicker';
import { useBoneMap } from './BoneMap.hooks';

interface BoneRowProps {
  bone: VrmBone;
  source: string | null;
  editing: boolean;
  saving: boolean;
  onEdit: () => void;
  onClose: () => void;
}

function BoneRow({ bone, source, editing, saving, onEdit, onClose }: BoneRowProps) {
  const label = boneLabel(bone.bone);
  const byHand = source === 'set by you' || source === 'left out by you';
  return (
    <li className="flex min-w-0 flex-col">
      <div className="flex min-w-0 items-center gap-2 py-0.5 text-meta">
        <Icon
          name={bone.joint ? 'check-circle' : bone.required ? 'x-circle' : 'circle'}
          size={14}
          className={cn(
            'flex-none',
            bone.joint ? 'text-sage' : bone.required ? 'text-danger' : 'text-fg-mute'
          )}
        />
        <span className="w-[136px] flex-none truncate text-fg-dim">{label}</span>
        <span
          title={bone.joint ?? undefined}
          className={cn(
            'min-w-0 flex-1 truncate font-mono text-[11.5px]',
            bone.joint ? 'text-fg' : 'text-fg-mute'
          )}
        >
          {bone.joint ?? 'missing'}
        </span>
        {source ? (
          <span
            className={cn(
              'flex-none rounded-pill border px-1.5 text-[11px] leading-4',
              byHand ? 'border-clay-text/40 text-clay-text' : 'border-line text-fg-mute'
            )}
          >
            {source}
          </span>
        ) : null}
        <IconButton
          icon={saving ? 'circle-notch' : 'note-pencil'}
          size="sm"
          label={`Pick the joint for ${label}`}
          aria-expanded={editing}
          disabled={saving}
          onClick={onEdit}
        />
      </div>
      {editing ? <BonePicker bone={bone.bone} onClose={onClose} /> : null}
    </li>
  );
}

/**
 * Which joint each VRM humanoid bone maps to: the 15 required, then the
 * rest. Each bone can be mapped by hand to any of the rig's joints, left
 * out, or put back to automatic; the override saves at once.
 */
export default function BoneMap() {
  const m = useBoneMap();
  const { readiness } = m;
  if (!readiness) return null;
  const groups = m.groups.map(group => {
    const { shown } = group;
    const rows = group.bones.map(b => (
      <BoneRow
        key={b.bone}
        bone={b}
        source={boneSource(b, m.overrides)}
        editing={m.editing === b.bone}
        saving={m.savingBone === b.bone}
        onEdit={() => m.edit(b.bone)}
        onClose={() => m.edit(null)}
      />
    ));
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
              onClick={() => m.toggle(group.id)}
            >
              {shown ? 'Hide' : 'Show'}
            </Button>
          ) : null}
        </div>
        {shown ? <ul className="m-0 grid list-none grid-cols-1 gap-x-4 p-0">{rows}</ul> : null}
      </div>
    );
  });
  const named = m.convention ? `Matched by ${m.convention} names. ` : '';
  const help =
    m.missing > 0
      ? `${named}${m.missing} required ${m.missing === 1 ? 'bone is' : 'bones are'} missing. Pick a joint for each with its pencil, or have an agent map them with set_avatar.`
      : m.unclear
        ? `${named}Some names are unclear: check the bones and pick the right joints.`
        : `${named}${readiness.required} of 15 required bones and ${readiness.mapped} of 55 in all. Pick a joint with a bone's pencil to change it.`;
  return (
    <Panel id="avatar-bones" title="Bone map" help={help}>
      {groups}
    </Panel>
  );
}
