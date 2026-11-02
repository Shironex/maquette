/**
 * The refs sheet's main page: the four view slots in send order with how
 * many are filled, the Other tray, Add refs and Make the other views, and
 * how a send uses the views.
 */
import { Button, Icon, InlineBanner, cn } from '@maquette/ui';
import type { AssetRef, RefWarning, RefsView } from '@/ipc/client';
import { SLOTS, VIEW_LABEL, viewsLine, warningsFor } from '@/state/refs';
import { Menu } from '@/features/common/Menu';
import { SectionTitle } from '../../SectionTitle';
import { ViewTile } from '../ViewTile';
import { useRefsBody } from './RefsBody.hooks';

export interface RefsBodyProps {
  view: RefsView;
  warnings: readonly RefWarning[];
  onSend: () => void;
}

export default function RefsBody({ view, warnings, onSend }: RefsBodyProps) {
  const body = useRefsBody(view);
  const { busy, front } = body;
  const tile = (item: AssetRef) => (
    <ViewTile
      key={item.file}
      item={item}
      projectDir={view.projectDir}
      refs={view.refs}
      warnings={warningsFor(warnings, item.file)}
      disabled={busy}
      {...body.actions(item)}
    />
  );
  const dots = SLOTS.map(v => (
    <span
      key={v}
      className={cn(
        'size-2 rounded-full',
        view.refs.some(r => r.view === v) ? 'bg-clay' : 'border border-line-strong'
      )}
    />
  ));
  const slots = SLOTS.map(slot => {
    const item = view.refs.find(r => r.view === slot);
    if (item) return tile(item);
    return (
      <li key={slot} className="flex min-w-0 flex-col gap-1">
        <button
          type="button"
          disabled={!front || busy}
          onClick={body.drawViews}
          className="flex aspect-square cursor-pointer flex-col items-center justify-center gap-1 rounded-[8px] border border-dashed border-line-strong text-meta text-fg-mute hover:border-clay hover:text-fg disabled:cursor-not-allowed disabled:opacity-60"
        >
          <span className="font-semibold text-fg-dim">{VIEW_LABEL[slot]}</span>
          {slot === 'front' ? 'Tag a front' : 'Empty, draw it'}
        </button>
        <span className="text-[11px] text-fg-mute">{VIEW_LABEL[slot]}</span>
      </li>
    );
  });
  const others = body.others.map(tile);
  const addItems = [
    {
      id: 'from-disk',
      label: 'Choose from disk...',
      icon: 'folder-open' as const,
      detail: 'Copies PNG, JPEG or WebP images into refs/',
      onSelect: () => void body.addFromDisk(),
    },
    ...body.free.map(i => ({
      id: i.file,
      label: i.name,
      icon: 'image' as const,
      onSelect: () => body.add(i.file),
    })),
  ];

  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        {body.readFromOrder ? (
          <InlineBanner
            title="Views read from the order of the refs"
            detail="The first ref is the front, then left, back and right, the way sends always went. Change a tag to keep the views on the asset."
          />
        ) : null}
        <section className="flex flex-col gap-2.5">
          <SectionTitle
            aside={
              <span className="inline-flex items-center gap-2">
                <span className="inline-flex gap-1" aria-hidden>
                  {dots}
                </span>
                {viewsLine(view.refs)}
              </span>
            }
          >
            Views, in send order
          </SectionTitle>
          <ul className="grid list-none grid-cols-4 gap-2 p-0">{slots}</ul>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              size="sm"
              icon="sparkle"
              disabled={!front || busy}
              disabledReason={
                front ? undefined : 'Tag a front first: the other views are drawn from it'
              }
              tooltip="Codex draws the missing views from the front, as candidates you pick from"
              onClick={body.drawViews}
            >
              {body.allFilled ? 'Draw a view again' : 'Make the other views'}
            </Button>
            {body.running > 0 ? (
              <span className="inline-flex items-center gap-1.5 text-meta text-fg-mute">
                <Icon name="circle-notch" spin size={12} /> {body.running} drawing in the Lab
              </span>
            ) : null}
          </div>
        </section>

        <section className="flex flex-col gap-2.5">
          <SectionTitle aside="Kept with the asset, never sent as views">Other</SectionTitle>
          {others.length === 0 ? (
            <p className="text-meta text-fg-mute">
              No other refs. Tag a ref Other to keep it here.
            </p>
          ) : (
            <ul className="grid list-none grid-cols-4 gap-2 p-0">{others}</ul>
          )}
          <div className="relative">
            <Button
              size="sm"
              icon="plus"
              disabled={busy}
              aria-expanded={body.adding}
              onClick={body.toggleAdding}
            >
              Add refs
            </Button>
            {body.adding ? (
              <Menu
                label="Add refs"
                onClose={body.stopAdding}
                className="absolute top-full left-0 mt-1 w-[260px]"
                items={addItems}
              />
            ) : null}
          </div>
        </section>

        <section className="flex flex-col gap-1.5 rounded-card border border-line bg-raised px-3 py-2.5 text-meta text-fg-dim">
          <span className="font-semibold text-fg">How a send uses the views</span>
          <span>Front goes first, then left, back and right; an empty view is skipped.</span>
          <span>Other refs stay home, even when they are picked in Send to 3D.</span>
          <span>
            By hand folders name the images by view: front.png, left.png, back.png, right.png.
          </span>
        </section>
      </div>
      <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
        <span className="mr-auto text-meta text-fg-mute">{body.footer}</span>
        <Button variant="secondary" onClick={body.close}>
          Done
        </Button>
        <Button variant="primary" icon="cube" disabled={!front} onClick={onSend}>
          Send to 3D
        </Button>
      </div>
    </>
  );
}
