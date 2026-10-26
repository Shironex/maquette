import { Button, EmptyState, Icon, NumberField, SegmentedControl, cn } from '@maquette/ui';
import { formatBytes, formatK } from '@/lib/format';
import { BY_HAND_REASON } from '@/state/game';
import { useStageLabels } from '@/state/use-stage-labels';
import { useBudgetPanel } from './BudgetPanel.hooks';

const folderName = (folder: string) => (folder ? `${folder}/` : 'top level');

/** Each model against its folder's budget, and the budget per folder. */
export default function BudgetPanel() {
  const b = useBudgetPanel();
  const stages = useStageLabels();
  if (!b.game) return null;
  const options = [
    { value: 'all', label: 'All' },
    { value: 'over', label: `Over budget${b.over.length ? ` ${b.over.length}` : ''}` },
    ...b.folders.map(f => ({ value: f, label: `${f}/` })),
  ];
  const rows = b.rows.map(r => {
    const stats = r.asset.versions[r.asset.versions.length - 1]?.stats;
    const overTris = r.check && r.check.triangles > r.check.maxTriangles;
    const overTex = r.check && r.check.texture > r.check.maxTexture;
    return (
      <tr key={r.asset.id} className="border-b border-line last:border-b-0">
        <td className="truncate py-1.5 pr-2 pl-3 font-medium text-fg" title={r.file}>
          {r.asset.name}
          <span className="ml-2 font-mono text-[11px] text-fg-mute">{r.file}</span>
        </td>
        <td className="px-2 py-1.5 text-fg-dim">{stages[r.asset.stage]}</td>
        <td
          className={cn('px-2 py-1.5 text-right font-mono tabular-nums', overTris && 'text-amber')}
        >
          {r.check ? `${formatK(r.check.triangles)} / ${formatK(r.check.maxTriangles)}` : '-'}
        </td>
        <td
          className={cn('px-2 py-1.5 text-right font-mono tabular-nums', overTex && 'text-amber')}
        >
          {r.check ? `${r.check.texture} / ${r.check.maxTexture}` : '-'}
        </td>
        <td className="px-2 py-1.5 text-right font-mono text-fg-dim tabular-nums">
          {stats?.bytes ? formatBytes(stats.bytes) : '-'}
        </td>
        <td
          className={cn('truncate py-1.5 pr-3 pl-2', r.fix ? 'text-amber' : 'text-sage')}
          title={r.byHand ? BY_HAND_REASON : undefined}
        >
          {r.check ? (r.fix ?? 'fits') : <span className="text-fg-mute">no model</span>}
        </td>
      </tr>
    );
  });

  const budgetRows = b.budgets.map(row => (
    <div key={row.folder} className="flex flex-col gap-1.5">
      <div className="flex items-baseline gap-2">
        <span className="font-mono text-[12px] font-semibold">{folderName(row.folder)}</span>
        {b.empty(row.folder) ? (
          <span className="text-meta text-fg-mute">no models from Maquette here</span>
        ) : null}
      </div>
      <div className="grid grid-cols-2 gap-2">
        <NumberField
          aria-label={`${folderName(row.folder)} triangles`}
          size="sm"
          value={row.triangles}
          min={100}
          unit="tris"
          format={formatK}
          onChange={triangles => b.edit(row.folder, { triangles })}
        />
        <NumberField
          aria-label={`${folderName(row.folder)} texture size`}
          size="sm"
          value={row.texture}
          min={64}
          max={16384}
          unit="px"
          onChange={texture => b.edit(row.folder, { texture })}
        />
      </div>
    </div>
  ));
  const overRows = b.over.map(r => (
    <li key={r.asset.id} className="flex flex-col gap-0.5">
      <span className="flex justify-between gap-2">
        <span className="truncate text-fg">{r.asset.name}</span>
        <span className="flex-none text-amber">{r.check?.problems.join(', ')}</span>
      </span>
      {r.byHand ? (
        <span className="flex items-start gap-1 text-fg-mute">
          <Icon name="person-simple-walk" size={12} className="mt-0.5 flex-none" />
          {BY_HAND_REASON}
        </span>
      ) : null}
    </li>
  ));
  return (
    <div className="flex min-h-0 flex-1 gap-5">
      <section className="flex min-w-0 flex-1 flex-col gap-3">
        <SegmentedControl
          label="Budget filter"
          options={options}
          value={b.filter}
          onChange={b.setFilter}
        />
        {b.total ? (
          <div className="min-h-0 flex-1 overflow-y-auto rounded-card border border-line bg-panel">
            <table className="w-full table-fixed border-collapse text-[12px]">
              <colgroup>
                <col />
                <col className="w-[84px]" />
                <col className="w-[110px]" />
                <col className="w-[100px]" />
                <col className="w-[76px]" />
                <col className="w-[84px]" />
              </colgroup>
              <thead>
                <tr className="border-b border-line text-[11px] tracking-[0.04em] text-fg-mute uppercase">
                  <th className="py-1.5 pr-2 pl-3 text-left font-medium">Model</th>
                  <th className="px-2 py-1.5 text-left font-medium">Stage</th>
                  <th className="px-2 py-1.5 text-right font-medium">Triangles</th>
                  <th className="px-2 py-1.5 text-right font-medium">Texture</th>
                  <th className="px-2 py-1.5 text-right font-medium">Size</th>
                  <th className="py-1.5 pr-3 pl-2 text-left font-medium">Budget</th>
                </tr>
              </thead>
              <tbody>{rows}</tbody>
            </table>
          </div>
        ) : (
          <EmptyState
            icon="cube"
            title="No models yet"
            body="Import the game's models from the Board first."
          />
        )}
      </section>
      <aside aria-label="Per-model budget" className="flex w-[300px] flex-none flex-col gap-3">
        <div>
          <h2 className="text-panel-title">Per-model budget</h2>
          <p className="text-meta text-fg-mute">
            Each model checks on its own. There is no scene total.
          </p>
        </div>
        <div className="flex flex-col gap-2 rounded-card border border-line bg-panel p-3">
          {budgetRows}
          {b.dirty ? (
            <div className="flex justify-end gap-2 pt-1">
              <Button size="sm" variant="ghost" onClick={b.reset}>
                Undo
              </Button>
              <Button size="sm" variant="primary" loading={b.saving} onClick={b.save}>
                Save budgets
              </Button>
            </div>
          ) : null}
        </div>
        <div className="flex flex-col gap-2 rounded-card border border-line bg-panel p-3">
          <h3 className="flex items-center gap-1.5 font-semibold">
            <Icon
              name="warning"
              size={14}
              className={b.over.length ? 'text-amber' : 'text-fg-mute'}
            />
            Over budget
          </h3>
          {b.over.length ? (
            <ul className="flex flex-col gap-1 text-meta">{overRows}</ul>
          ) : (
            <p className="text-meta text-fg-mute">Every model fits its folder&apos;s budget.</p>
          )}
          <Button
            icon="wrench"
            disabled={!b.fixable}
            disabledReason={
              b.over.length ? 'Rigged models are reduced by hand' : 'Nothing is over budget'
            }
            onClick={b.addFixups}
          >
            {b.fixable === 1 ? 'Add the fix-up' : `Add the ${b.fixable} fix-ups`}
          </Button>
          <p className="text-meta text-fg-mute">
            Decimate or Shrink textures, applied by Build models on a copy. The repo file stays v1.
            Rigged models never get Decimate, only Shrink textures.
          </p>
        </div>
      </aside>
    </div>
  );
}
