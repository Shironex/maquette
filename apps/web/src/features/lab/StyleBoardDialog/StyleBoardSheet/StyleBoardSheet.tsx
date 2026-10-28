import { Button, Icon, IconButton, TextField } from '@maquette/ui';
import type { LabView } from '@/ipc/client';
import { KEYS } from '@/lib/keys';
import { MAX_STYLE_LINE } from '@/state/lab-style';
import type { StyleDraft } from '@/state/use-lab-store';
import { PaletteField } from './PaletteField';
import { StyleImages } from './StyleImages';
import { useStyleBoardSheet } from './StyleBoardSheet.hooks';

const CARRIES = [
  "The line and palette, after each direction's own prompt",
  "The images, attached after the direction's own references",
  'The brief still leads where the two disagree',
  'Each save is a new revision; a job keeps the one it started with',
];

export interface StyleBoardSheetProps {
  draft: StyleDraft;
  view: LabView;
}

/** The dialog itself: the images, the style line, the palette, and Save. */
export default function StyleBoardSheet({ draft, view }: StyleBoardSheetProps) {
  const {
    setStyle,
    close,
    busy,
    ref,
    changed,
    empty,
    addImage,
    removeImage,
    clear,
    submit,
    onKeyDown,
  } = useStyleBoardSheet(draft, view);
  const revision = view.style.revision;
  const carries = CARRIES.map(line => (
    <li key={line} className="flex gap-1.5 text-meta text-fg-dim">
      <Icon name="check" size={13} className="mt-[3px] flex-none text-sage" />
      {line}
    </li>
  ));

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="style-title"
        onKeyDown={onKeyDown}
        className="flex max-h-full w-[680px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
      >
        <div className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="palette" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="style-title" className="truncate text-panel-title">
              Style board
            </h2>
            <p className="truncate text-meta text-fg-mute">
              {revision > 0
                ? `Revision ${revision} · every new direction and object ref carries it`
                : 'Every new direction and object ref will carry it'}
            </p>
          </div>
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={close} />
        </div>

        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4">
          <StyleImages view={view} images={draft.images} onAdd={addImage} onRemove={removeImage} />
          <TextField
            label="Style line"
            value={draft.line}
            maxLength={MAX_STYLE_LINE}
            placeholder="frosted glass, cool night light, chunky silhouettes"
            hint="One line about the look: materials, light, shapes. Not what to show."
            onChange={e => setStyle({ line: e.target.value })}
          />
          <PaletteField palette={draft.palette} onChange={palette => setStyle({ palette })} />
          <div className="flex flex-col gap-1.5 rounded-card border border-line bg-raised px-3 py-2.5">
            <p className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
              What Codex gets
            </p>
            <ul className="m-0 flex list-none flex-col gap-1 p-0">{carries}</ul>
          </div>
        </div>

        <div className="flex items-center gap-2 border-t border-line px-5 py-3">
          <Button
            variant="ghost"
            icon="trash"
            disabled={empty}
            disabledReason="The board is empty"
            tooltip="Empties the draft; Save makes the board empty, so jobs carry nothing"
            onClick={clear}
          >
            Clear
          </Button>
          <span className="flex-1" />
          <Button variant="ghost" onClick={close}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon="check"
            loading={busy}
            disabled={!changed || busy}
            disabledReason="Nothing changed yet"
            tooltip="Saves the board as a new revision for the next directions"
            shortcut={KEYS.submit}
            onClick={submit}
          >
            Save board
          </Button>
        </div>
      </div>
    </div>
  );
}
