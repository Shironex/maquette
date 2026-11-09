import { Button, NumberField, Select } from '@maquette/ui';
import { useTrimForm } from './TrimForm.hooks';

/** One clip's trim: which of its frames play, what that keeps, and the time edits it leaves out. */
export default function TrimForm() {
  const f = useTrimForm();
  if (!f) return null;
  const outside = f.outside.join(', ');
  return (
    <div
      role="group"
      aria-label="Trim settings"
      className="flex flex-col gap-2.5 border-t border-line px-3 py-3"
    >
      {f.clips.length > 1 ? (
        <Select
          label="Clip"
          size="sm"
          options={f.clips}
          value={String(f.index)}
          onChange={f.choose}
        />
      ) : null}
      <div className="grid grid-cols-2 gap-2">
        <NumberField
          label="In"
          size="sm"
          unit="f"
          min={0}
          max={f.b - 1}
          value={f.a}
          onChange={f.setIn}
        />
        <NumberField
          label="Out"
          size="sm"
          unit="f"
          min={f.a + 1}
          max={f.clip.frames}
          value={f.b}
          onChange={f.setOut}
        />
      </div>
      <p className="font-mono text-[11.5px] text-fg-dim">{f.keptLine}</p>
      {f.outside.length ? (
        <p className="text-meta text-amber">
          {`Outside the trim: ${outside}. ${f.outside.length === 1 ? 'It stays' : 'They stay'} in the recipe and play again when the trim takes those frames back.`}
        </p>
      ) : null}
      {f.problem ? (
        <p role="alert" className="text-meta text-danger">
          {f.problem}
        </p>
      ) : null}
      <p className="text-meta text-fg-mute">
        Nothing is thrown away. The trim only picks which frames play, and the clip keeps all of its
        keys.
      </p>
      <div>
        <Button size="sm" variant="ghost" disabled={!f.clip.trim} onClick={f.keepAll}>
          Keep every frame
        </Button>
      </div>
    </div>
  );
}
