import { Button, Checkbox, EmptyState, Icon, IconButton, cn } from '@maquette/ui';
import { homePath } from '@/lib/format';
import { KEYS } from '@/lib/keys';
import { checkText, useGameImportDialog } from './GameImportDialog.hooks';

const TONE = { ok: 'text-sage', over: 'text-amber', note: 'text-fg-dim' } as const;

/** Import models: the game's own models, to pick which ones Maquette looks after. */
export default function GameImportDialog() {
  const d = useGameImportDialog();
  if (!d.open) return null;
  const rows = d.models.map(m => {
    const check = checkText(m);
    const done = Boolean(m.asset);
    const name = m.file.split('/').pop() ?? m.file;
    return (
      <tr key={m.file} className={cn('border-b border-line last:border-b-0', done && 'opacity-60')}>
        <td className="py-1.5 pr-2 pl-3">
          <Checkbox
            checked={done || d.picked.has(m.file)}
            onChange={on => !done && d.toggle(m.file, on)}
            label={<span className="sr-only">Import {m.file}</span>}
          />
        </td>
        <td className="truncate px-2 py-1.5 font-medium text-fg" title={m.file}>
          {name}
        </td>
        <td className="truncate px-2 py-1.5 font-mono text-fg-dim">
          {m.folder ? `${m.folder}/` : '/'}
        </td>
        <td className="px-2 py-1.5 font-mono text-fg-dim">.{m.format}</td>
        <td className="px-2 py-1.5 text-right tabular-nums">{d.triangles(m)}</td>
        <td className="px-2 py-1.5 text-right tabular-nums text-fg-dim">{d.textures(m)}</td>
        <td className="px-2 py-1.5 text-right tabular-nums text-fg-dim">{d.size(m)}</td>
        <td className={cn('truncate py-1.5 pr-3 pl-2', TONE[check.tone])} title={check.text}>
          {check.text}
        </td>
      </tr>
    );
  });
  const stat = (label: string, value: string, tone?: string) => (
    <div className="flex flex-col rounded-control border border-line bg-raised px-3 py-1.5">
      <span className="text-caption tracking-[0.06em] text-fg-mute uppercase">{label}</span>
      <span className={cn('font-semibold tabular-nums', tone)}>{value}</span>
    </div>
  );

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => e.target === e.currentTarget && d.close()}
    >
      <form
        ref={d.sheet}
        role="dialog"
        aria-modal="true"
        aria-labelledby="game-import-title"
        className="flex max-h-full w-[880px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
        onSubmit={e => {
          e.preventDefault();
          d.submit();
        }}
      >
        <header className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="download-simple" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="game-import-title" className="truncate text-panel-title">
              Import models from {d.name}
            </h2>
            <p className="truncate text-meta text-fg-mute">
              Choose the models Maquette should look after. It never changes the ones you leave out.
            </p>
          </div>
          <IconButton icon="x" label="Close" shortcut={KEYS.escape} onClick={d.close} />
        </header>
        <div className="flex min-h-0 flex-1 flex-col gap-3 px-5 py-4">
          <p className="flex items-center gap-2 text-meta text-fg-mute">
            <Icon name="game-controller" size={14} />
            Game
            <span className="truncate font-mono text-fg-dim">
              {d.repo ? homePath(d.repo, d.home) : 'no game repo'}
            </span>
          </p>
          <div className="grid grid-cols-4 gap-2.5">
            {stat('Found', d.counts.found)}
            {stat('Selected', d.counts.selected, 'text-clay-text')}
            {stat('Over budget', d.counts.over, d.counts.over !== '0' ? 'text-amber' : undefined)}
            {stat('On disk', d.counts.disk)}
          </div>
          <p className="text-meta text-fg-mute">
            {d.scanned} Each model becomes an asset with the repo file as v1. Fix-ups and Compare
            then work on it.
          </p>
          {d.scanning ? (
            <p className="text-meta text-fg-mute">Reading the models</p>
          ) : d.models.length ? (
            <div className="min-h-0 flex-1 overflow-y-auto rounded-card border border-line">
              <table className="w-full table-fixed border-collapse text-[12px]">
                <colgroup>
                  <col className="w-[40px]" />
                  <col />
                  <col className="w-[96px]" />
                  <col className="w-[60px]" />
                  <col className="w-[76px]" />
                  <col className="w-[72px]" />
                  <col className="w-[72px]" />
                  <col className="w-[168px]" />
                </colgroup>
                <thead className="sticky top-0 bg-panel">
                  <tr className="border-b border-line text-[11px] tracking-[0.04em] text-fg-mute uppercase">
                    <th className="py-1.5 pr-2 pl-3 text-left font-medium">
                      <Checkbox
                        checked={d.picked.size > 0}
                        onChange={d.pickAll}
                        label={<span className="sr-only">Pick every model</span>}
                      />
                    </th>
                    <th className="px-2 py-1.5 text-left font-medium">Model</th>
                    <th className="px-2 py-1.5 text-left font-medium">Folder</th>
                    <th className="px-2 py-1.5 text-left font-medium">Format</th>
                    <th className="px-2 py-1.5 text-right font-medium">Triangles</th>
                    <th className="px-2 py-1.5 text-right font-medium">Textures</th>
                    <th className="px-2 py-1.5 text-right font-medium">Size</th>
                    <th className="py-1.5 pr-3 pl-2 text-left font-medium">Check</th>
                  </tr>
                </thead>
                <tbody>{rows}</tbody>
              </table>
            </div>
          ) : (
            <EmptyState
              icon="cube"
              title="No models found"
              body={`There is no .glb or .obj in ${d.scan?.modelsDir ?? 'the model folder'}/. Check the model folder in Settings.`}
            />
          )}
          <p className="flex items-center gap-1.5 text-meta text-fg-mute">
            <Icon name="info" size={14} />
            Imported files join the ship record, so Ship may replace them later. Over budget models
            import as they are.
          </p>
        </div>
        <footer className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button variant="ghost" onClick={d.close}>
            Back
          </Button>
          <Button
            type="submit"
            variant="primary"
            icon="download-simple"
            loading={d.importing}
            disabled={!d.picked.size || d.importing}
            disabledReason="Pick at least one model"
          >
            Import {d.counts.selected} {d.picked.size === 1 ? 'model' : 'models'}
          </Button>
        </footer>
      </form>
    </div>
  );
}
