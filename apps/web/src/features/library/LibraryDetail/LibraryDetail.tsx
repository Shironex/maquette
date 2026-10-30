import type { ReactNode } from 'react';
import { Button, KeyValueList, Select, StatusChip, VersionBadge } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { SOURCE_LABEL, numbersLine, versionLine } from '@/state/library';
import { LibraryPicture } from '../LibraryPicture';
import { TagEditor } from '../TagEditor';
import { useLibraryDetail } from './LibraryDetail.hooks';

export interface LibraryDetailProps {
  item: LibraryItem;
}

const KIND_LABEL = { prop: 'Prop', character: 'Character', scene: 'Scene' } as const;

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
        {title}
      </h3>
      {children}
    </div>
  );
}

/** The selected model: Use in project, its versions, where it is used and its credits. */
export default function LibraryDetail({ item }: LibraryDetailProps) {
  const d = useLibraryDetail(item);
  const { entry } = item;
  const subtitle = [
    KIND_LABEL[entry.kind],
    numbersLine(item),
    item.format ? `.${item.format}` : null,
  ]
    .filter(Boolean)
    .join(' · ');
  const usesOf = (n: number) => item.uses.filter(u => u.version === n).length;
  const versions = [...entry.versions].reverse().map(v => (
    <li key={v.version} className="flex items-center gap-2.5 text-meta">
      <VersionBadge version={v.version} latest={v.version === d.newest} />
      <span className="min-w-0 flex-1 truncate text-fg-dim">{versionLine(v, d.now)}</span>
      <span className="font-mono text-[11px] text-fg-mute">
        {usesOf(v.version) === 0 ? 'none' : `${usesOf(v.version)} used`}
      </span>
    </li>
  ));
  const uses = item.uses.map(u => (
    <li key={`${u.project}/${u.asset}`} className="flex items-center gap-2.5">
      <span className="min-w-0 flex-1">
        <b className="block truncate font-semibold">{u.projectName}</b>
        <span className="block truncate text-meta text-fg-mute">
          v{u.version} · {u.assetName}
        </span>
      </span>
      {u.update ? (
        <Button
          size="sm"
          icon="arrow-up"
          tooltip="Brings this project's copy up to the newest library version, at no credits"
          onClick={() => d.update({ project: u.project, asset: u.asset })}
        >
          Update to v{d.newest}
        </Button>
      ) : (
        <StatusChip status="done" label="current" icon={false} />
      )}
    </li>
  ));
  const projectOptions = d.projects.map(p => ({ value: p.id, label: `Add to ${p.name}` }));
  const versionOptions = [...entry.versions]
    .reverse()
    .map(v => ({ value: String(v.version), label: `v${v.version}` }));

  return (
    <aside
      aria-label="Selected model"
      className="flex w-[360px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel p-[18px]"
    >
      <LibraryPicture
        item={item}
        onInspect={d.quickView}
        className="aspect-[4/3] w-full flex-none rounded-card border border-line"
      />
      <div>
        <h2 className="text-panel-title">{entry.name}</h2>
        <p className="mt-0.5 text-meta text-fg-mute">{subtitle}</p>
      </div>
      <div className="flex flex-col gap-2">
        <div className="flex items-end gap-2">
          <Select
            label="Use in project"
            options={projectOptions}
            value={d.projectId}
            onChange={d.setProjectId}
            fieldClassName="min-w-0 flex-1"
          />
          {entry.versions.length > 1 ? (
            <Select
              label="Version"
              options={versionOptions}
              value={String(d.version)}
              onChange={v => d.setVersion(Number(v))}
              fieldClassName="w-[76px]"
            />
          ) : null}
        </div>
        <Button
          variant="primary"
          icon="copy"
          disabled={Boolean(d.blocked) || d.busy}
          disabledReason={d.blocked ?? 'Working on it'}
          tooltip="Copies this library model into the project, at no credits"
          loading={d.busy}
          onClick={d.useIt}
        >
          Add, no credits
        </Button>
        <p className="text-meta text-fg-mute">{d.blocked ? `${d.blocked}.` : d.note}</p>
      </div>
      <div className="h-px bg-line" />
      <Section title="Tags">
        <TagEditor entry={entry} />
      </Section>
      <div className="h-px bg-line" />
      <Section title={`Versions · ${entry.versions.length}`}>
        <ul className="flex list-none flex-col gap-2">{versions}</ul>
      </Section>
      <div className="h-px bg-line" />
      <Section title={`Where it is used · ${item.uses.length}`}>
        {uses.length ? (
          <ul className="flex list-none flex-col gap-2.5">{uses}</ul>
        ) : (
          <p className="text-meta text-fg-mute">No project uses it yet.</p>
        )}
      </Section>
      <div className="h-px bg-line" />
      <Section title="Credits">
        <KeyValueList items={d.credits} />
        <p className="text-meta text-fg-mute">
          Made with {SOURCE_LABEL[entry.versions.at(-1)?.source ?? 'imported']}. Adding, using and
          updating copies spend nothing.
        </p>
      </Section>
    </aside>
  );
}
