import { IconButton, SegmentedControl } from '@maquette/ui';
import type { ReactNode } from 'react';
import { useChoiceDialog } from '@/features/common/ChoiceDialog/ChoiceDialog.hooks';
import { useChoice } from './RigDoctor.hooks';

export interface EnlargedPicture {
  url: string | null;
  caption: string;
}

/** The pictures seen together, such as before and draft from the front. */
export interface EnlargedPage {
  label: string;
  pictures: readonly EnlargedPicture[];
}

function EnlargeDialog({
  title,
  detail,
  pages,
  onClose,
}: {
  title: string;
  detail: string;
  pages: readonly EnlargedPage[];
  onClose: () => void;
}) {
  const ref = useChoiceDialog(onClose);
  const [label, setLabel] = useChoice(pages[0]?.label ?? '');
  const pictures = pages.find(p => p.label === label)?.pictures ?? [];
  const shown = pictures.map(p => (
    <figure key={p.caption} className="m-0 flex min-h-0 min-w-0 flex-col gap-1">
      <div className="min-h-0 flex-1 overflow-hidden rounded-[8px] border border-line bg-studio">
        {p.url ? (
          <img
            src={p.url}
            alt={`${title}, ${p.caption}`}
            className="h-full w-full object-contain"
          />
        ) : null}
      </div>
      <figcaption className="text-meta text-fg-mute">{p.caption}</figcaption>
    </figure>
  ));
  return (
    <div
      className="fixed inset-0 z-[60] flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={`${title}, enlarged`}
        className="flex h-full max-h-[900px] w-full max-w-[1200px] flex-col gap-3 rounded-card border border-line-strong bg-panel p-5 shadow-window"
      >
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h2 className="m-0 font-mono text-panel-title">{title}</h2>
            <p className="m-0 mt-0.5 text-meta text-fg-dim">{detail}</p>
          </div>
          <div className="flex flex-none items-center gap-2">
            {pages.length > 1 ? (
              <SegmentedControl
                label="View"
                tone="neutral"
                options={pages.map(p => ({ value: p.label, label: p.label }))}
                value={label}
                onChange={setLabel}
              />
            ) : null}
            <IconButton icon="x" label="Close" autoFocus onClick={onClose} />
          </div>
        </div>
        <div
          className="grid min-h-0 flex-1 auto-rows-fr gap-3"
          style={{ gridTemplateColumns: `repeat(${pictures.length}, minmax(0, 1fr))` }}
        >
          {shown}
        </div>
      </div>
    </div>
  );
}

/** Pictures of a row that open bigger on a click: before and after together in a comparison. */
export function Enlarges({
  title,
  detail,
  pages,
  children,
}: {
  title: string;
  detail: string;
  pages: readonly EnlargedPage[];
  children: ReactNode;
}) {
  const [open, setOpen] = useChoice(false);
  return (
    <>
      <button
        type="button"
        aria-label={`Enlarge ${title}`}
        className="block w-full cursor-zoom-in rounded-[8px] text-left"
        onClick={() => setOpen(true)}
      >
        {children}
      </button>
      {open ? (
        <EnlargeDialog title={title} detail={detail} pages={pages} onClose={() => setOpen(false)} />
      ) : null}
    </>
  );
}
