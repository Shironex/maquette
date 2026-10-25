import { Button, Field, Icon, NumberStepper, Select, TextArea, TextField, cn } from '@maquette/ui';
import type { Aspect, LabView } from '@/ipc/client';
import { ASPECTS, MAX_IMAGES } from '@/state/lab';
import { EarlierDirections } from '../EarlierDirections';
import { References } from './References';
import { useComposer } from './Composer.hooks';

export interface ComposerProps {
  view: LabView;
}

/** The left panel: the brief for a new direction (or more of an earlier one). */
export default function Composer({ view }: ComposerProps) {
  const { draft, setDraft, generate, busy, newDraft, codex, existing, ready, codexReady } =
    useComposer(view);
  const n = draft.count;

  return (
    <section
      aria-label="New direction"
      className="flex w-[320px] flex-none flex-col gap-3.5 overflow-y-auto border-r border-line bg-panel px-[18px] pt-[18px] pb-4"
    >
      <div className="flex items-center justify-between gap-2">
        <h1 className="flex items-center gap-2 text-panel-title">
          <Icon name={existing ? 'images' : 'plus'} size={14} className="text-clay-text" />
          {existing ? 'Add to direction' : 'New direction'}
        </h1>
        {existing ? (
          <Button size="sm" variant="ghost" icon="plus" onClick={newDraft}>
            New
          </Button>
        ) : null}
      </div>
      <TextField
        label="Direction name"
        value={draft.name}
        maxLength={60}
        placeholder="Moon island porch"
        onChange={e => setDraft({ name: e.target.value })}
      />
      <TextArea
        label="Prompt"
        rows={6}
        value={draft.prompt}
        placeholder="What should the concepts show? Setting, mood, light, the props that matter."
        onChange={e => setDraft({ prompt: e.target.value })}
      />
      <References view={view} />
      <div className="grid grid-cols-2 gap-2.5">
        <Select<Aspect>
          label="Aspect"
          options={ASPECTS}
          value={draft.aspect}
          onChange={aspect => setDraft({ aspect })}
        />
        <Field id="lab-count" label="Images">
          <div className="[&>span]:h-[34px] [&>span]:w-full [&>span>span]:min-w-0 [&>span>span]:flex-1">
            <NumberStepper
              label="Images"
              value={n}
              min={1}
              max={MAX_IMAGES}
              onChange={count => setDraft({ count })}
            />
          </div>
        </Field>
      </div>
      {view.directions.length > 0 ? <EarlierDirections view={view} /> : null}
      <div className="mt-auto flex flex-col gap-2 pt-1">
        <Button
          variant="primary"
          size="lg"
          block
          iconRight="arrow-right"
          loading={busy === 'generate'}
          disabled={!ready || busy === 'generate' || !codexReady}
          title={ready ? undefined : 'Name the direction and write a prompt first'}
          onClick={() => void generate()}
        >
          {`Generate ${n} ${n === 1 ? 'concept' : 'concepts'}`}
        </Button>
        <span className={cn('text-center text-meta', codexReady ? 'text-fg-mute' : 'text-amber')}>
          {codexReady
            ? `Runs through Codex CLI, ${view.concurrency} at a time`
            : `Codex CLI is not ready. ${codex?.hint ?? 'Install it, then run codex login.'}`}
        </span>
      </div>
    </section>
  );
}
