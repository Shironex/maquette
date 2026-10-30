import { IconButton, TextField, cn } from '@maquette/ui';
import { KEYS } from '@/lib/keys';
import { boneLabel } from '@/state/avatar';
import { jointDomId, useBonePicker } from './BonePicker.hooks';

export interface BonePickerProps {
  /** The VRM bone being mapped, such as `leftUpperLeg`. */
  bone: string;
  onClose: () => void;
}

/**
 * Picks the joint one VRM bone maps to: a search over the rig's joints,
 * with Automatic (map it by name again) and Leave unmapped first. A pick
 * saves at once and the readiness reads again.
 */
export default function BonePicker({ bone, onClose }: BonePickerProps) {
  const p = useBonePicker(bone, onClose);
  const label = boneLabel(bone);
  const rows = p.choices.map(choice => {
    const active = p.active?.id === choice.id;
    return (
      <div
        key={choice.id}
        id={jointDomId(p.listId, choice)}
        role="option"
        aria-selected={active}
        className={cn(
          'flex cursor-pointer items-center gap-2 px-2.5 py-1.5 text-meta',
          active ? 'bg-clay-soft' : 'hover:bg-studio'
        )}
        onPointerDown={e => e.preventDefault()}
        onClick={() => p.pick(choice)}
      >
        <span
          className={cn(
            'min-w-0 flex-1 truncate',
            typeof choice.value === 'string' ? 'font-mono text-[11.5px] text-fg' : 'text-fg-dim'
          )}
        >
          {choice.label}
        </span>
        {choice.takenBy ? (
          <span className="flex-none truncate text-[11px] text-fg-mute">
            now {boneLabel(choice.takenBy).toLowerCase()}
          </span>
        ) : null}
      </div>
    );
  });
  return (
    <div
      ref={p.ref}
      className="mt-1 mb-2 flex flex-col gap-1.5 rounded-card border border-line-strong bg-raised p-2"
      onKeyDown={p.onKey}
    >
      <div className="flex items-center gap-1.5">
        <TextField
          role="combobox"
          aria-label={`Search the joints for ${label}`}
          aria-expanded="true"
          aria-controls={p.listId}
          aria-autocomplete="list"
          aria-activedescendant={p.active ? jointDomId(p.listId, p.active) : undefined}
          aria-busy={p.saving}
          placeholder="Search joints"
          icon="magnifying-glass"
          autoComplete="off"
          value={p.query}
          onChange={e => p.search(e.target.value)}
          fieldClassName="min-w-0 flex-1"
        />
        <IconButton
          icon="x"
          size="sm"
          label="Close the joint list"
          shortcut={KEYS.escape}
          onClick={onClose}
        />
      </div>
      <div
        id={p.listId}
        role="listbox"
        aria-label={`Joints for ${label}`}
        className="max-h-[220px] min-h-0 overflow-y-auto overscroll-contain rounded-control border border-line bg-panel"
      >
        {rows.length === 0 ? (
          <p className="px-2.5 py-3 text-center text-meta text-fg-mute">
            No joint matches “{p.query.trim()}”.
          </p>
        ) : (
          rows
        )}
      </div>
    </div>
  );
}
