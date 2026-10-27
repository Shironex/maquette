import { Button, NumberField, Select, TextField } from '@maquette/ui';
import { SectionLabel } from '@/features/common/SectionLabel';
import { usePoseEditor } from './PoseEditor.hooks';

const AXES = ['X', 'Y', 'Z'] as const;
const round = (v: number) => String(Math.round(v * 1000) / 1000);

/** Names the selected state and poses its parts, as offsets from their rest pose. */
export default function PoseEditor() {
  const editor = usePoseEditor();
  if (!editor) return null;
  const { state, rest, pose } = editor;
  if (rest) {
    return (
      <section aria-label="Pose" className="flex flex-col gap-2">
        <SectionLabel>{state.name}</SectionLabel>
        <p className="text-meta text-fg-mute">
          The first state is the model as its file has it, every part at rest. Add a state to pose
          its parts.
        </p>
      </section>
    );
  }
  const fields = (['position', 'rotation'] as const).map(key => {
    const axes = AXES.map((axis, i) => (
      <NumberField
        key={axis}
        label={`${key === 'position' ? 'Move' : 'Turn'} ${axis}`}
        size="sm"
        unit={key === 'position' ? 'm' : '°'}
        min={key === 'position' ? -100 : -360}
        max={key === 'position' ? 100 : 360}
        format={round}
        value={pose[key][i]!}
        onChange={v => editor.set(key, i, v)}
      />
    ));
    return (
      <div key={key} className="grid grid-cols-3 gap-2">
        {axes}
      </div>
    );
  });
  const partOptions = editor.parts.map(p => ({
    value: p,
    label: editor.posed.includes(p) ? `${p}, moved` : p,
  }));
  return (
    <section aria-label="Pose" className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <SectionLabel>Pose in {state.name}</SectionLabel>
        <Button
          size="sm"
          variant="ghost"
          icon="trash"
          tooltip="Deletes this state"
          onClick={editor.remove}
        >
          Remove state
        </Button>
      </div>
      <TextField
        key={state.id}
        label="Name"
        defaultValue={state.name}
        onBlur={e => editor.rename(e.target.value)}
        onKeyDown={e => {
          if (e.key === 'Enter') e.currentTarget.blur();
        }}
      />
      {editor.part ? (
        <>
          <Select
            label="Part"
            size="sm"
            options={partOptions}
            value={editor.part}
            onChange={editor.setPart}
          />
          {fields}
        </>
      ) : (
        <p className="text-meta text-fg-mute">The model file has no parts to pose.</p>
      )}
    </section>
  );
}
