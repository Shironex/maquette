import { Button, Icon, InlineBanner } from '@maquette/ui';
import { blankFilled } from '@/state/recipe-draft';
import { RecipeBlankForm } from '../RecipeBlankForm';
import { useRecipeBlanks } from './RecipeBlanks.hooks';

/**
 * The targets each project fills: a prop of the project or a point. Changes
 * aim at them by id; the preview marks each filled one.
 */
export default function RecipeBlanks() {
  const b = useRecipeBlanks();
  const rows = b.blanks.map(({ blank, fill }) => {
    const open = b.picked === blank.id;
    const problem = b.problems[blank.id];
    const filled = blankFilled(blank);
    return (
      <li key={blank.id} className="rounded-card border border-line bg-raised">
        <button
          type="button"
          aria-expanded={open}
          onClick={() => b.toggle(blank.id)}
          className="flex w-full items-center gap-2.5 px-3 py-2 text-left"
        >
          <Icon
            name={blank.kind === 'prop' ? 'cube' : 'map-pin'}
            size={16}
            className={`flex-none ${blank.kind === 'prop' ? 'text-clay' : 'text-sage'}`}
          />
          <span className="min-w-0 flex-1">
            <span className="block truncate text-[13px] font-medium">{blank.label}</span>
            <span className={`block truncate text-meta ${filled ? 'text-fg-mute' : 'text-amber'}`}>
              {fill}
            </span>
            {problem && !open ? (
              <span className="block truncate text-meta text-danger">{problem}</span>
            ) : null}
          </span>
          <span className="font-mono text-[11px] text-fg-mute">{blank.id}</span>
          <Icon name={open ? 'caret-up-down' : 'caret-down'} size={14} className="text-fg-mute" />
        </button>
        {open ? <RecipeBlankForm id={blank.id} /> : null}
      </li>
    );
  });
  return (
    <section aria-label="Blanks" className="flex flex-col gap-2">
      <div className="flex items-end gap-2">
        <h3 className="flex-1 text-[13px] font-semibold">
          Blanks <span className="font-normal text-fg-mute">{b.blanks.length}</span>
        </h3>
        <Button
          size="sm"
          icon="cube"
          disabled={Boolean(b.addBlocked)}
          disabledReason={b.addBlocked}
          onClick={() => b.add('prop')}
        >
          Add prop
        </Button>
        <Button
          size="sm"
          icon="map-pin"
          disabled={Boolean(b.addBlocked)}
          disabledReason={b.addBlocked}
          onClick={() => b.add('point')}
        >
          Add point
        </Button>
      </div>
      {b.warnings.length ? (
        <InlineBanner tone="warning" title="A prop is gone" detail={b.warnings.join(' ')} />
      ) : null}
      {b.blanks.length ? (
        <ul aria-label="Recipe blanks" className="flex list-none flex-col gap-1.5">
          {rows}
        </ul>
      ) : (
        <p className="text-meta text-fg-mute">
          No blanks yet. A reach or a look at aims at one: a prop of the project or a point.
        </p>
      )}
    </section>
  );
}
