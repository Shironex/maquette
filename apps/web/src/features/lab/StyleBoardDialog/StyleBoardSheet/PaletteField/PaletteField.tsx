import { Button, IconButton, TextField } from '@maquette/ui';
import { MAX_PALETTE, normalizeColour } from '@/state/lab-style';
import { usePaletteField } from './PaletteField.hooks';

export interface PaletteFieldProps {
  palette: readonly string[];
  onChange: (next: string[]) => void;
}

/** The board's colours as swatches with Remove, a typed hex or the colour picker to add one. */
export default function PaletteField({ palette, onChange }: PaletteFieldProps) {
  const { text, type, bad, commit, onKey, remove } = usePaletteField(palette, onChange);
  const full = palette.length >= MAX_PALETTE;
  const swatches = palette.map(colour => (
    <li
      key={colour}
      className="group flex h-8 items-center gap-1.5 rounded-pill border border-line bg-raised pr-1 pl-1.5"
    >
      <span
        aria-hidden="true"
        className="size-5 rounded-full shadow-[0_0_0_1px_var(--color-line-strong)]"
        style={{ background: colour }}
      />
      <span className="font-mono text-[12px]">{colour}</span>
      <IconButton icon="x" label={`Remove ${colour}`} size="sm" onClick={() => remove(colour)} />
    </li>
  ));

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-baseline justify-between">
        <span className="font-medium">Palette</span>
        <span className="text-meta text-fg-mute tabular-nums">
          {palette.length} of {MAX_PALETTE}
        </span>
      </div>
      {swatches.length ? (
        <ul className="m-0 flex list-none flex-wrap gap-1.5 p-0">{swatches}</ul>
      ) : null}
      {full ? null : (
        <div className="flex items-start gap-2">
          <input
            type="color"
            aria-label="Pick a colour"
            value={normalizeColour(text) ?? '#9fc3ff'}
            onChange={e => type(e.target.value)}
            className="h-[34px] w-10 flex-none cursor-pointer rounded-control border border-line-strong bg-app p-1"
          />
          <TextField
            aria-label="Colour"
            value={text}
            mono
            placeholder="#9fc3ff"
            fieldClassName="min-w-0 flex-1"
            note={
              bad ? { tone: 'missing', text: 'Write a colour as #rrggbb, or #rgb.' } : undefined
            }
            onChange={e => type(e.target.value)}
            onKeyDown={onKey}
          />
          <Button
            icon="plus"
            disabled={!text.trim()}
            disabledReason="Type a colour first"
            onClick={() => commit(text)}
          >
            Add
          </Button>
        </div>
      )}
      <p className="text-meta text-fg-mute">Optional. A few colours Codex keeps to.</p>
    </div>
  );
}
