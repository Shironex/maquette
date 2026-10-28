/**
 * Save clips to the library from a rigged model: its newest version, or a
 * Tripo job's animations in the Inbox. Only the skeleton and the chosen
 * clips' keys go in; clips that came from the library stay out.
 */
import { Button, Checkbox, Icon, InlineBanner, TextField } from '@maquette/ui';
import type { SaveClip } from '@/ipc/client';
import { Sheet } from '@/features/common/Sheet';
import { formatBytes } from '@/lib/format';
import { RIG_LABEL } from '@/state/animations';
import { useSaveClipsSheet } from './SaveClipsSheet.hooks';

const RIG_NOTE =
  'Read from the bone names. Library clips from this rig map onto other rigs with the same bone map step.';

function clipLine(c: SaveClip): string {
  return [
    `${c.frames} f`,
    `${Math.round(c.fps ?? 0)} fps`,
    c.loops ? 'loop' : null,
    c.rootTravel ? 'root' : null,
  ]
    .filter(Boolean)
    .join(' · ');
}

export default function SaveClipsSheet() {
  const v = useSaveClipsSheet();
  const { sheet, plan, chosen } = v;
  if (!sheet) return null;
  const rows = plan?.clips.map(c =>
    c.fromLibrary ? (
      <li key={c.name} className="flex items-center gap-2.5 py-1 text-meta text-fg-mute">
        <Icon name="books" size={15} />
        <span className="min-w-0 flex-1">
          <b className="font-semibold text-fg-dim">{c.name}</b>
          <span className="block">Came from the library ({c.fromLibrary})</span>
        </span>
        <span className="font-mono text-[11px]">{formatBytes(c.bytes ?? 0)}</span>
      </li>
    ) : (
      <li key={c.name} className="flex items-center gap-2.5 py-1">
        <Checkbox
          checked={chosen.includes(c.name)}
          onChange={on => v.toggle(c.name, on)}
          label={
            <span>
              <b className="font-semibold">{c.name}</b>
              <span className="block font-mono text-[11.5px] text-fg-mute">{clipLine(c)}</span>
            </span>
          }
          className="min-w-0 flex-1"
        />
        <span className="font-mono text-[11px] text-fg-mute">{formatBytes(c.bytes ?? 0)}</span>
      </li>
    )
  );
  const tags = sheet.tags.map(t => (
    <span
      key={t}
      className="rounded-pill border border-line px-1.5 font-mono text-[11px] leading-[18px] text-fg-dim"
    >
      {t}
    </span>
  ));
  return (
    <Sheet
      id="save-clips"
      icon="books"
      title="Save clips to the library"
      meta={v.meta}
      onClose={v.close}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4">
        {sheet.error ? (
          <InlineBanner tone="danger" title="Could not save the clips" detail={sheet.error} />
        ) : null}
        {plan ? (
          <>
            <div className="flex items-start gap-2.5 rounded-card border border-line bg-raised p-3">
              <Icon name="bone" size={18} className="mt-0.5 text-fg-dim" />
              <span className="min-w-0 flex-1">
                <b className="block font-semibold">
                  {RIG_LABEL[plan.rig]} rig, {plan.bones} bones
                </b>
                <span className="block text-meta text-fg-mute">{RIG_NOTE}</span>
              </span>
              <span className="rounded-pill bg-sage-soft px-2 text-[11px] leading-[18px] font-medium text-sage">
                detected
              </span>
            </div>
            <section aria-label={`Clips on ${plan.name}`} className="flex flex-col gap-1">
              <h3 className="flex text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
                <span className="flex-1">
                  Clips on {plan.name} · {plan.clips.length}
                </span>
                <span>{v.count} chosen</span>
              </h3>
              <ul className="flex list-none flex-col">{rows}</ul>
            </section>
            <TextField
              label="Pack"
              value={sheet.pack}
              maxLength={80}
              hint={v.newPack ? 'New pack' : 'Adds to this pack'}
              onChange={e => v.change({ pack: e.target.value })}
            />
            {sheet.tags.length ? (
              <div className="flex flex-col gap-1.5">
                <span className="text-meta text-fg-mute">Tags · suggested from the clip names</span>
                <p className="flex flex-wrap gap-1">{tags}</p>
              </div>
            ) : null}
            <InlineBanner
              tone="info"
              title="Skeleton and keys only."
              detail={`The library gets ${plan.bones} bones and ${v.clips}, about ${formatBytes(v.bytes)}. The ${formatBytes(plan.modelBytes ?? 0)} mesh and its textures stay where they are.`}
            />
          </>
        ) : null}
      </div>
      <div className="flex items-center gap-3 border-t border-line px-5 py-3.5">
        <span className="min-w-0 flex-1 truncate text-meta text-fg-mute">
          <b className="text-[15px] font-semibold text-fg">{v.clips}</b> to {sheet.pack || 'a pack'}
        </span>
        <Button onClick={v.close}>Cancel</Button>
        <Button
          variant="primary"
          icon="books"
          disabled={!plan || v.count === 0 || !sheet.pack.trim() || v.busy}
          disabledReason={
            !plan
              ? 'Still reading the model'
              : v.count === 0
                ? 'Pick a clip first'
                : 'Name the pack'
          }
          loading={v.busy}
          onClick={v.run}
        >
          Save {v.clips}
        </Button>
      </div>
    </Sheet>
  );
}
