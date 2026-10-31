import { Icon, cn } from '@maquette/ui';
import type { RigChain, RigRecord } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { DefectCard, RunProgress, Stats, Tile } from './parts';

const READ_TONE: Record<string, string> = {
  chain: 'text-fg-dim',
  stray: 'text-clay-text',
  swapped: 'text-amber',
  junk: 'text-clay-text',
};

const READ_WORD: Record<string, string> = {
  chain: 'Chain',
  stray: 'Stray weight',
  swapped: 'Sides swapped',
  junk: 'Junk',
};

function chainWord(c: RigChain): string {
  return c.note ?? READ_WORD[c.readAs] ?? c.readAs;
}

/** The centre of the third board: the chains read from the bones. */
export function CreatureCentre({ record }: { record: RigRecord }) {
  const creature = record.creature;
  const rows = record.before?.rows ?? [];
  const chains = (creature?.chains ?? []).map(c => (
    <tr key={c.chain} className="border-t border-line">
      <td className="py-1.5 font-mono text-fg">{c.chain}</td>
      <td className="py-1.5 font-mono text-fg-dim">{c.bones}</td>
      <td className="py-1.5 font-mono text-fg-dim">{c.side}</td>
      <td className={cn('py-1.5', READ_TONE[c.readAs] ?? 'text-fg-dim')}>{chainWord(c)}</td>
    </tr>
  ));
  const pairs = rows
    .filter(r => r.note)
    .slice(0, 2)
    .map(row => {
      const after = record.after?.rows.find(r => r.id === row.id);
      return (
        <figure key={row.id} className="m-0">
          <div className="grid h-[132px] grid-cols-2 gap-1.5">
            <Tile url={fileUrl(row.front)} alt={`${row.label}, before`} flagged />
            <Tile url={fileUrl(after?.front)} alt={`${row.label}, draft`} />
          </div>
          <figcaption className="mt-1 flex justify-between font-mono text-meta">
            <span className="text-fg-dim">{row.label}</span>
            <span className="text-clay-text">{row.note}</span>
          </figcaption>
        </figure>
      );
    });
  const cards = record.defects.map(d => <DefectCard key={d.number} defect={d} />);
  const log = record.log.slice(-6).map(l => (
    <li key={`${l.at}-${l.text}`} className="flex gap-2.5">
      <span className="flex-none text-fg-mute">{l.at}</span>
      <span className="flex-none text-clay-text">{l.script}</span>
      <span className="min-w-0 truncate text-fg-dim">{l.text}</span>
    </li>
  ));
  return (
    <section
      aria-label="Chains"
      className="flex min-w-0 flex-1 flex-col gap-3 overflow-y-auto px-5 pt-4 pb-4"
    >
      <div>
        <h2 className="m-0 text-panel-title">
          {record.name}{' '}
          <span className="ml-1 rounded-[5px] bg-hover px-1.5 py-0.5 font-mono text-meta">
            v{record.source.version ?? 1}
          </span>
        </h2>
        <p className="m-0 mt-0.5 text-fg-mute">
          Its own bone names{record.startedBy === 'agent' ? ' · started by an agent over MCP' : ''}.
          Names stay as they are.
        </p>
      </div>
      <h3 className="m-0 text-caption tracking-[0.06em] text-fg-mute uppercase">
        Chains, filled from the bone tree
      </h3>
      <table className="w-full flex-none border-collapse text-meta">
        <thead>
          <tr className="text-left text-caption tracking-[0.06em] text-fg-mute uppercase">
            <th className="py-1 font-medium">Chain</th>
            <th className="py-1 font-medium">Bones</th>
            <th className="py-1 font-medium">Side</th>
            <th className="py-1 font-medium">Read as</th>
          </tr>
        </thead>
        <tbody>{chains}</tbody>
      </table>
      <div className="grid flex-none grid-cols-2 gap-3">{pairs}</div>
      <div className="flex flex-none flex-col gap-2">{cards}</div>
      <RunProgress record={record} />
      {record.log.length > 0 ? (
        <ol
          aria-label="Log"
          className="m-0 flex list-none flex-col gap-0.5 p-0 font-mono text-meta"
        >
          {log}
        </ol>
      ) : null}
      {record.stats.length > 0 ? <Stats stats={record.stats} /> : null}
    </section>
  );
}

/** The right column of the third board: who started it, the map and the run. */
export function CreatureAside({ record }: { record: RigRecord }) {
  const map = fileUrl(record.creature?.weightMap);
  const stages = record.stages.map(s => {
    const going = s.seconds === null;
    const took = Math.round(s.seconds ?? 0);
    return (
      <li key={s.name} className="flex items-start gap-2">
        <Icon
          name={going ? 'circle-notch' : 'check-circle'}
          size={14}
          className={cn('mt-0.5 flex-none', going ? 'animate-spin text-clay-text' : 'text-sage')}
        />
        <span className="min-w-0 flex-1">
          <span className="block text-fg">{s.name}</span>
          <span className="block text-fg-mute">{s.detail}</span>
        </span>
        {going ? null : <span className="flex-none font-mono text-fg-mute">{took} s</span>}
      </li>
    );
  });
  return (
    <aside
      aria-label="Headless Blender"
      className="flex w-[300px] flex-none flex-col gap-3 overflow-y-auto border-l border-line p-4"
    >
      {record.startedBy === 'agent' ? (
        <div className="rounded-[9px] border border-line bg-raised p-3 text-meta text-fg-mute">
          <p className="m-0 mb-1 flex items-center gap-1.5 font-medium text-fg">
            <Icon name="robot" size={14} /> Started over MCP
          </p>
          An agent asked for this. It can read the sheets, but only you can add the result.
        </div>
      ) : null}
      <div>
        <h3 className="m-0 mb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
          Weight map
        </h3>
        <div className="h-[170px]">
          <Tile url={map} alt="Bone weights in colour" />
        </div>
      </div>
      <div>
        <h3 className="m-0 mb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
          Headless Blender
        </h3>
        <ul className="m-0 flex list-none flex-col gap-1.5 p-0 text-meta">{stages}</ul>
      </div>
    </aside>
  );
}
