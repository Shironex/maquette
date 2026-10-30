import type { ReactNode } from 'react';
import { Select, TextField, Toggle } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import { KIND_GROUP, NEW_COLLECTION } from '@/state/add-many';
import { useAddManySheet } from './AddManySheet.hooks';
import AddManyFooter from './AddManyFooter';
import ClashRow from './ClashRow';

function Block({ title, note, children }: { title: string; note?: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-baseline gap-2">
        <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
          {title}
        </h3>
        {note ? <span className="text-meta text-fg-mute">{note}</span> : null}
      </div>
      {children}
    </section>
  );
}

/**
 * Add a whole project to the library: the project tag, a collection, tags
 * by kind, the models already in the library with a choice each, the files
 * that could not be read, and the progress while it copies.
 */
export default function AddManySheet() {
  const s = useAddManySheet();
  const { plan, options, progress, sum, busy } = s;
  const kindRows = sum?.kinds.map(([kind, n]) => (
    <div key={kind} className="flex items-center gap-3">
      <span className="w-28 flex-none font-semibold">{KIND_GROUP[kind]}</span>
      <span className="w-20 flex-none text-meta text-fg-mute">
        {n} {n === 1 ? 'model' : 'models'}
      </span>
      <TextField
        aria-label={`Tags for ${KIND_GROUP[kind].toLowerCase()}`}
        value={options.kindTags[kind]}
        maxLength={120}
        disabled={busy}
        onChange={e => s.setKindTags(kind, e.target.value)}
        fieldClassName="flex-1"
      />
    </div>
  ));
  const clashRows = s.clashes.map(p => (
    <ClashRow
      key={p.asset}
      planned={p}
      choice={s.choices[p.asset] ?? 'rename'}
      disabled={busy}
      onChoose={c => s.choose(p.asset, c)}
    />
  ));
  const unreadableRows = sum?.unreadable.map(p => (
    <li key={p.asset} className="text-fg-dim">
      <b className="text-fg">{p.file.split('/').pop()}</b> {p.reason ?? 'could not be read'}
    </li>
  ));

  return (
    <Sheet
      id="add-many"
      icon="books"
      title={s.title}
      meta="Every accepted model in the project, in one go. Names and tags are set before anything is copied."
      onClose={s.close}
      width={560}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        <Select
          label="Project"
          options={s.projectOptions}
          value={s.project ?? ''}
          disabled={busy}
          onChange={id => id && void s.pick(id)}
        />
        {s.error ? (
          <p role="alert" className="text-meta text-danger">
            {s.error}
          </p>
        ) : null}
        {plan && sum ? (
          <>
            <div className="flex items-end gap-3">
              <TextField
                label="Project tag"
                value={options.tag}
                maxLength={32}
                disabled={!options.projectTag || busy}
                onChange={e => s.setOptions({ tag: e.target.value })}
                fieldClassName="flex-1"
              />
              <Toggle
                label="On every model"
                checked={options.projectTag}
                disabled={busy}
                onChange={projectTag => s.setOptions({ projectTag })}
                className="mb-2"
              />
            </div>
            <div className="flex items-end gap-3">
              <Select
                label="Collection"
                options={s.collectionOptions}
                value={options.collection}
                disabled={busy}
                onChange={collection => s.setOptions({ collection })}
                fieldClassName="flex-1"
              />
              {options.collection === NEW_COLLECTION ? (
                <TextField
                  label="Name"
                  value={options.newCollection}
                  maxLength={60}
                  disabled={busy}
                  onChange={e => s.setOptions({ newCollection: e.target.value })}
                  fieldClassName="flex-1"
                />
              ) : null}
            </div>
            <Block title="Versions">
              <p className="text-meta text-fg-dim">
                The newest accepted model of each; older versions stay in the project.
              </p>
            </Block>
            <Block title="Tags by kind" note="from the asset kinds">
              {kindRows}
            </Block>
            {sum.clashes.length || sum.sameFile.length ? (
              <Block
                title="Already in the library"
                note={`${sum.clashes.length} to decide, ${sum.sameFile.length} skipped`}
              >
                {sum.sameFile.length ? (
                  <p className="text-meta text-fg-dim">
                    {sum.sameFile.length === 1
                      ? '1 is the same file and is'
                      : `${sum.sameFile.length} are the same file and are`}{' '}
                    skipped.
                  </p>
                ) : null}
                <ul className="flex list-none flex-col gap-1.5">{clashRows}</ul>
              </Block>
            ) : null}
            {sum.unreadable.length ? (
              <Block title={`${sum.unreadable.length} could not be read`}>
                <ul className="flex list-none flex-col gap-1 text-meta">{unreadableRows}</ul>
                <p className="text-meta text-fg-mute">
                  They stay in the project; add them after a redo.
                </p>
              </Block>
            ) : null}
          </>
        ) : null}
      </div>
      <AddManyFooter
        progress={progress}
        result={s.result}
        count={s.count}
        ready={plan !== null}
        onStart={() => void s.start()}
        onStop={s.stop}
        onClose={s.close}
      />
    </Sheet>
  );
}
