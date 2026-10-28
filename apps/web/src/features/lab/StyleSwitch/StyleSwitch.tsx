import { Button, Icon, IconButton, Toggle, cn } from '@maquette/ui';
import type { LabView } from '@/ipc/client';
import { useStyleSwitch } from './StyleSwitch.hooks';

export interface StyleSwitchProps {
  view: LabView;
  /** Whether this direction carries the project style. */
  on: boolean;
  onChange: (on: boolean) => void;
  /** Without a board, offers to set one up; else shows nothing. */
  offerSetup?: boolean;
}

/** The project style as a brief carries it: what the board holds, and the switch that turns it off for this one. */
export default function StyleSwitch({ view, on, onChange, offerSetup = false }: StyleSwitchProps) {
  const { board, empty, counts, thumbs, openStyle } = useStyleSwitch(view);
  if (empty) {
    if (!offerSetup) return null;
    return (
      <div className="flex items-center gap-2.5 rounded-card border border-dashed border-line-strong px-3 py-2.5">
        <Icon name="palette" size={16} className="flex-none text-fg-mute" />
        <p className="min-w-0 flex-1 text-meta text-fg-mute">
          A style board keeps every direction in one look.
        </p>
        <Button size="sm" variant="ghost" onClick={openStyle}>
          Set up
        </Button>
      </div>
    );
  }
  const pictures = thumbs.map(t => (
    <span key={t.file} className="block size-7 flex-none overflow-hidden rounded-[6px] bg-raised">
      {t.url ? <img src={t.url} alt="" className="size-full object-cover" /> : null}
    </span>
  ));
  const swatches = board.palette.map(colour => (
    <span
      key={colour}
      className="size-3 flex-none rounded-full shadow-[0_0_0_1px_var(--color-line-strong)]"
      style={{ background: colour }}
    />
  ));
  return (
    <div
      className={cn(
        'flex flex-col gap-2 rounded-card border px-3 py-2.5 transition-colors duration-120',
        on ? 'border-clay-line bg-clay-soft/40' : 'border-line bg-raised'
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <Toggle
          checked={on}
          onChange={onChange}
          label="Use the project style"
          description={
            on
              ? `Revision ${board.revision}, after your prompt`
              : 'This direction keeps its own look'
          }
        />
        <IconButton
          icon="palette"
          label="Open the style board"
          hint="The line, palette and images every new direction carries"
          size="sm"
          onClick={openStyle}
          className="-mt-0.5 flex-none text-fg-mute"
        />
      </div>
      <div className={cn('flex flex-col gap-1.5', !on && 'opacity-55')}>
        {board.line ? (
          <span className="line-clamp-2 text-meta text-fg-dim" title={board.line}>
            {board.line}
          </span>
        ) : null}
        <span className="flex items-center gap-2" title={counts}>
          {pictures.length ? <span className="flex gap-1">{pictures}</span> : null}
          {swatches.length ? <span className="flex gap-1">{swatches}</span> : null}
          <span className="sr-only">{counts}</span>
        </span>
      </div>
    </div>
  );
}
