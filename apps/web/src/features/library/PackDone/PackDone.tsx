import type { ReactNode } from 'react';
import { Button, Icon } from '@maquette/ui';
import type { PackRecord } from '@/ipc/client';
import { shortPath } from '@/lib/format';
import PackPicture from '../PackCheck/PackPicture';
import { RecentPacks } from '../RecentPacks';
import { usePackDone } from './PackDone.hooks';

function Block({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section aria-label={title} className="flex min-w-0 flex-col gap-2">
      <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
        {title}
      </h3>
      {children}
    </section>
  );
}

export interface PackDoneProps {
  record: PackRecord;
}

/**
 * A finished pack: where it is, what it weighs and how far its triangles
 * came down, what is inside, the manifest and contact sheet, how the check
 * opened each model again, and the packs before it.
 */
export default function PackDone({ record }: PackDoneProps) {
  const d = usePackDone(record);
  const stats = d.stats.map(s => (
    <div
      key={s.label}
      className="flex flex-col rounded-card border border-line bg-raised px-3 py-2.5"
    >
      <dt className="text-meta text-fg-mute">{s.label}</dt>
      <dd className="text-[20px] leading-tight font-semibold">{s.value}</dd>
      <span className="text-meta text-fg-mute">{s.note}</span>
    </div>
  ));
  const tree = d.tree.map((row, n) => (
    <li
      key={`${row.path}-${n}`}
      className="flex gap-2 py-0.5"
      style={{ paddingLeft: row.depth * 16 }}
    >
      <span className="min-w-0 flex-1 truncate">{row.path}</span>
      {row.note ? <span className="text-fg-mute">{row.note}</span> : null}
    </li>
  ));
  const imports = d.imports.map(row => (
    <li key={row.label} className="flex items-center gap-2">
      <Icon
        name={row.opened === row.of ? 'check-circle' : 'warning'}
        size={16}
        className={row.opened === row.of ? 'text-sage' : 'text-amber'}
      />
      {row.label}, {row.opened} of {row.of}
    </li>
  ));
  const pictures = d.kept.map(item => (
    <li
      key={item.model.key}
      className="grid aspect-square place-items-center overflow-hidden rounded-[4px] bg-studio"
    >
      <PackPicture item={item} />
    </li>
  ));
  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        <div className="flex items-start gap-3">
          <p className="min-w-0 flex-1 text-fg-dim">{d.line}</p>
          <Button size="sm" icon="copy" onClick={() => void d.copy()}>
            Copy path
          </Button>
          <Button size="sm" icon="folder-open" disabled={d.busy} onClick={() => void d.reveal()}>
            {d.revealLabel}
          </Button>
        </div>
        <dl className="grid grid-cols-4 gap-3">{stats}</dl>
        <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-5">
          <Block title="What is inside">
            <ul className="flex max-h-[260px] list-none flex-col overflow-y-auto rounded-card border border-line bg-app px-3 py-2 font-mono text-[12px]">
              <li className="flex gap-2 py-0.5 font-semibold">
                <Icon name={record.zip ? 'package' : 'folder'} size={14} className="mt-0.5" />
                {record.name}
                {record.zip ? '.zip' : '/'}
              </li>
              {tree}
            </ul>
          </Block>
          <Block title="Contact sheet">
            {d.sheet ? (
              <img
                src={d.sheet}
                alt={`Every model in ${record.name}`}
                className="rounded-card border border-line"
                onError={d.sheetFailed}
              />
            ) : (
              <ul
                aria-label={`Every model in ${record.name}`}
                className="grid list-none grid-cols-6 gap-1 rounded-card border border-line bg-studio p-1.5"
              >
                {pictures}
              </ul>
            )}
          </Block>
        </div>
        <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-5">
          {d.manifest ? (
            <Block title="manifest.json">
              <pre className="overflow-x-auto rounded-card border border-line bg-app p-3 font-mono text-[11px] leading-[1.5] text-fg-dim">
                {d.manifest}
              </pre>
            </Block>
          ) : null}
          <Block title="Opened again">
            <ul className="flex list-none flex-col gap-1.5">
              {imports}
              <li className="flex items-center gap-2">
                <Icon name="check-circle" size={16} className="text-sage" />Y up, size and pivot as
                the manifest says
              </li>
              <li className="flex items-center gap-2 text-fg-mute">
                <Icon name="info" size={16} />
                Each file is opened in an empty scene before it is zipped.
              </li>
            </ul>
          </Block>
        </div>
        <Block title={`Earlier packs · ${shortPath(d.where, 2)}`}>
          <RecentPacks
            recent={d.recent}
            current={record.id}
            busy={d.busy}
            onOpen={id => void d.showRecord(id)}
            onResume={id => void d.resume(id)}
          />
          <p className="text-meta text-fg-mute">
            Start a pack from the Library (a selection or a collection), from a project on its Board
            (every accepted model), or from an agent (export_pack with a preset).
          </p>
        </Block>
      </div>
      <div className="flex items-center gap-2 border-t border-line px-5 py-3.5">
        <span
          className="min-w-0 flex-1 truncate font-mono text-meta text-fg-dim"
          title={record.zip ?? record.dir}
        >
          {record.zip ?? record.dir}
        </span>
        <Button onClick={d.again}>Another pack</Button>
      </div>
    </>
  );
}
