import { Button, EmptyState, Icon, cn } from '@maquette/ui';
import type { ShipFile } from '@/ipc/client';
import { commitFirstLine, originalsLine, shipCounts } from '@/state/game';
import { shipCells, splitPath, useShipView } from './ShipView.hooks';

export interface ShipViewProps {
  project: string;
  /** Back to the runner. */
  onClose: () => void;
}

const MARK = { new: '+', replaced: '~', unchanged: '=', 'hand-made': '·' } as const;
const MARK_TONE = {
  new: 'text-sage',
  replaced: 'text-amber',
  unchanged: 'text-fg-mute',
  'hand-made': 'text-fg-mute',
} as const;
const UNCHANGED_SHOWN = 4;

function FileRow({ file, why }: { file: ShipFile; why?: string }) {
  const [folder, name] = splitPath(file.path);
  const cells = shipCells(file);
  return (
    <tr className="border-t border-line">
      <td className={cn('w-6 py-1.5 pl-3 font-mono', MARK_TONE[file.change])} aria-hidden="true">
        {MARK[file.change]}
      </td>
      <td className="truncate py-1.5 pr-2 font-mono text-[12px]" title={file.path}>
        <span className="text-fg-mute">{folder}</span>
        {name}
      </td>
      <td className="px-2 py-1.5 font-mono text-[11.5px] text-fg-dim tabular-nums">{cells.size}</td>
      <td className="px-2 py-1.5 text-fg-dim">{cells.model}</td>
      <td className="py-1.5 pr-3 pl-2 text-fg-mute">{why ?? ''}</td>
    </tr>
  );
}

function Group({ title, note, count, children }: GroupProps) {
  return (
    <tbody>
      <tr>
        <th
          colSpan={5}
          className="bg-raised py-1.5 pl-3 text-left text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase"
        >
          {title} <span className="ml-1 tabular-nums">{count}</span>
          {note ? (
            <span className="ml-2 font-normal tracking-normal normal-case">{note}</span>
          ) : null}
        </th>
      </tr>
      {children}
    </tbody>
  );
}

interface GroupProps {
  title: string;
  note?: string;
  count: number;
  children: React.ReactNode;
}

/** What Ship writes into the game repo, and the approval it waits for. */
export default function ShipView({ project, onClose }: ShipViewProps) {
  const { plan, groups, approval, deciding, answer, name, repo, showAll, setShowAll } =
    useShipView(project);
  if (!plan || !groups) {
    return <main className="grid flex-1 place-items-center" aria-busy="true" />;
  }
  const changing = groups.added.length + groups.replaced.length;
  const total = plan.files.length + plan.handMade.length;
  const unchanged = showAll ? groups.unchanged : groups.unchanged.slice(0, UNCHANGED_SHOWN);
  const subtitle = [
    approval ? `Run ${approval.runNumber} · waiting for you` : 'What the next Ship changes',
    `${changing} of ${total} files change`,
  ].join(' · ');
  const rows = (files: ShipFile[], why?: (f: ShipFile) => string) =>
    files.map(f => <FileRow key={f.path} file={f} why={why?.(f)} />);
  const added = rows(groups.added, () => 'new to the game');
  const replaced = rows(groups.replaced, f =>
    f.imported ? 'replaces the imported original' : 'budget fix-ups'
  );
  const unchangedRows = rows(unchanged);
  const blocked = rows(groups.blocked, () => 'left alone');
  const originals = originalsLine(groups);
  const commitFirst = commitFirstLine(plan);
  const commitNote = commitFirst ? (
    <p role="note" className="flex gap-1.5 text-meta text-amber">
      <Icon name="git-branch" size={14} className="mt-0.5 flex-none" />
      {commitFirst}
    </p>
  ) : null;
  const handMade = rows(groups.handMade);
  const more = groups.unchanged.length - UNCHANGED_SHOWN;
  const filesWord = changing === 1 ? 'file' : 'files';
  const checks = plan.checks.map(c => (
    <li key={c.text} className="flex items-center gap-1.5">
      <Icon
        name={c.ok ? 'check-circle' : 'warning-circle'}
        size={14}
        className={c.ok ? 'text-sage' : 'text-amber'}
      />
      {c.text}
    </li>
  ));

  return (
    <main className="flex min-h-0 flex-1 gap-5 overflow-y-auto px-6 pt-[18px] pb-[18px]">
      <div className="flex min-w-0 flex-1 flex-col gap-3">
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1">
            <h1 className="text-screen-title">Ship to {name}</h1>
            <p className="mt-0.5 truncate text-fg-mute">{subtitle}</p>
          </div>
          <Button
            variant="ghost"
            icon="arrow-left"
            tooltip="Closes this and shows the pipeline runner again"
            onClick={onClose}
          >
            Back to the runner
          </Button>
        </div>
        {!plan.exported ? (
          <EmptyState
            icon="rocket-launch"
            title="Nothing exported yet"
            body="Run the pipeline first. Ship copies what Game export wrote."
          />
        ) : (
          <div className="rounded-card border border-line bg-panel">
            <table className="w-full table-fixed border-collapse text-[12px]">
              <colgroup>
                <col className="w-7" />
                <col />
                <col className="w-[160px]" />
                <col className="w-[130px]" />
                <col className="w-[200px]" />
              </colgroup>
              <thead className="sr-only">
                <tr>
                  <th>Change</th>
                  <th>File</th>
                  <th>Size</th>
                  <th>Model</th>
                  <th>Why</th>
                </tr>
              </thead>
              <Group title="New" count={groups.added.length}>
                {added}
              </Group>
              <Group
                title="Replaced"
                note="Maquette wrote or imported these before"
                count={groups.replaced.length}
              >
                {replaced}
              </Group>
              <Group title="Unchanged" count={groups.unchanged.length}>
                {unchangedRows}
                {more > 0 ? (
                  <tr className="border-t border-line">
                    <td colSpan={5} className="py-1 pl-9">
                      <Button size="sm" variant="ghost" onClick={() => setShowAll(!showAll)}>
                        {showAll ? 'Show fewer' : `Show all, ${more} more`}
                      </Button>
                    </td>
                  </tr>
                ) : null}
              </Group>
              {groups.blocked.length ? (
                <Group
                  title="Held back"
                  note="A hand made file is already there"
                  count={groups.blocked.length}
                >
                  {blocked}
                </Group>
              ) : null}
              <Group
                title="Hand made, left alone"
                note="Not in the ship record, so Ship never touches them"
                count={groups.handMade.length}
              >
                {handMade}
              </Group>
            </table>
          </div>
        )}
      </div>

      <aside className="flex w-[320px] flex-none flex-col gap-3.5">
        {approval ? (
          <section
            role="alertdialog"
            aria-labelledby="ship-approval-title"
            className="flex flex-col gap-3 rounded-card border border-amber/40 bg-panel p-3.5"
          >
            <div className="flex items-start gap-3">
              <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-amber-soft text-amber">
                <Icon name="rocket-launch" size={17} />
              </span>
              <div className="min-w-0 flex-1">
                <h2 id="ship-approval-title" className="text-emphasis leading-5 font-semibold">
                  Ship {name}, run {approval.runNumber}
                </h2>
                <p className="mt-0.5 text-meta text-fg-mute">
                  Everything before Ship is done. Ship writes {changing} {filesWord} to{' '}
                  {plan.modelsDir}/ in the game repo.
                </p>
              </div>
            </div>
            <ul className="m-0 flex list-none flex-col gap-1 p-0 text-meta">
              <li>{shipCounts(groups)}</li>
              {originals ? (
                <li className="text-amber">
                  {originals}: the game&apos;s own files as they were before Maquette.
                </li>
              ) : null}
              <li className="text-fg-mute">Hand made files stay as they are.</li>
            </ul>
            {commitNote}
            <div className="flex justify-end gap-2">
              <Button
                variant="secondary"
                disabled={deciding}
                tooltip="Does not ship; nothing is written to the game repo"
                onClick={() => void answer(false)}
              >
                Deny
              </Button>
              <Button
                variant="primary"
                icon="check"
                loading={deciding}
                disabled={deciding}
                tooltip="Writes these files to the game repo"
                onClick={() => void answer(true)}
              >
                Approve and ship
              </Button>
            </div>
          </section>
        ) : null}
        <section className="flex flex-col gap-2 rounded-card border border-line bg-panel p-3.5">
          <h2 className="text-[13px] font-semibold">Where it writes</h2>
          <dl className="m-0 grid grid-cols-[88px_minmax(0,1fr)] gap-x-2 gap-y-1 text-meta">
            <dt className="text-fg-mute">Game repo</dt>
            <dd className="m-0 truncate font-mono" title={plan.repo ?? ''}>
              {repo ?? 'not set'}
            </dd>
            <dt className="text-fg-mute">Folder</dt>
            <dd className="m-0 font-mono">{plan.modelsDir}/</dd>
            <dt className="text-fg-mute">Branch</dt>
            <dd className="m-0">{plan.branch ?? 'not a git repo'}</dd>
            <dt className="text-fg-mute">Ship record</dt>
            <dd className="m-0">{plan.recordAfter} files after this run</dd>
          </dl>
          <p className="text-meta text-fg-dim">
            Ship never commits. Review the change in git and commit it yourself.
          </p>
          {approval ? null : commitNote}
        </section>
        <section className="flex flex-col gap-2 rounded-card border border-line bg-panel p-3.5">
          <h2 className="text-[13px] font-semibold">Checks before Ship</h2>
          <ul className="m-0 flex list-none flex-col gap-1.5 p-0 text-meta">{checks}</ul>
        </section>
      </aside>
    </main>
  );
}
