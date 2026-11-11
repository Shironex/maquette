import { Button, Icon, Select, cn, type IconName } from '@maquette/ui';
import type { LayerKind } from '@/ipc/client';
import { layerLine } from '@/state/recipe-draft';
import { RecipeChangeForm } from '../RecipeChangeForm';
import { useRecipeChanges } from './RecipeChanges.hooks';

const KIND_ICON: Record<LayerKind, IconName> = {
  reach: 'hand-pointing',
  nudge: 'sliders-horizontal',
  'look-at': 'eye',
  wave: 'person-arms-spread',
  pin: 'map-pin',
  pose: 'bone',
  grip: 'hand-pointing',
};

/** The changes laid over the base clip, in order; one opens to edit at a time. */
export default function RecipeChanges() {
  const c = useRecipeChanges();
  const rows = c.layers.map(layer => {
    const open = c.picked === layer.id;
    const problem = c.problems[layer.id];
    const drafted = c.drafted(layer);
    return (
      <li
        key={layer.id}
        className={cn(
          'rounded-card border border-line bg-raised',
          drafted && 'border-agent/50 shadow-[inset_3px_0_0_var(--color-agent)]'
        )}
      >
        <button
          type="button"
          aria-expanded={open}
          onClick={() => c.toggle(layer.id)}
          className="flex w-full items-center gap-2.5 px-3 py-2 text-left"
        >
          <Icon name={KIND_ICON[layer.kind]} size={16} className="flex-none text-fg-dim" />
          <span className="min-w-0 flex-1">
            <span className="block truncate text-[13px] font-medium">
              {layerLine(layer, c.toFrame)}
            </span>
            {problem && !open ? (
              <span className="block truncate text-meta text-danger">{problem}</span>
            ) : null}
          </span>
          {drafted ? (
            <span className="flex-none rounded-full bg-agent/15 px-1.5 py-0.5 text-[11px] font-medium text-agent">
              {drafted}
            </span>
          ) : null}
          <span className="font-mono text-[11px] text-fg-mute">{layer.id}</span>
          <Icon name={open ? 'caret-up-down' : 'caret-down'} size={14} className="text-fg-mute" />
        </button>
        {open ? <RecipeChangeForm id={layer.id} /> : null}
      </li>
    );
  });
  return (
    <section aria-label="Changes" className="flex flex-col gap-2">
      <div className="flex items-end gap-2">
        <h3 className="flex-1 text-[13px] font-semibold">
          Changes <span className="font-normal text-fg-mute">{c.layers.length}</span>
        </h3>
        <Select
          aria-label="Kind of change"
          size="sm"
          options={c.kinds}
          value={c.kind}
          onChange={(k: LayerKind) => c.setKind(k)}
          title={c.kindAbout}
          fieldClassName="w-[120px]"
        />
        <Button
          size="sm"
          icon="plus"
          disabled={Boolean(c.addBlocked)}
          disabledReason={c.addBlocked}
          onClick={c.add}
        >
          {c.kind === 'grip' ? 'Add a grip' : 'Add change'}
        </Button>
      </div>
      {c.layers.length ? (
        <ul aria-label="Recipe changes" className="flex list-none flex-col gap-1.5">
          {rows}
        </ul>
      ) : (
        <p className="text-meta text-fg-mute">
          No changes yet. Add a reach, a nudge, a look at or a wave; the preview shows it roughly.
        </p>
      )}
    </section>
  );
}
