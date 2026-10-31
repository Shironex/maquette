import { NumberField, Toggle } from '@maquette/ui';
import type { AssetKind, KindRow, PackSettings } from '@/ipc/client';
import { formatCount } from '@/lib/format';
import { MIN_BUDGET } from '@/state/packs';

export interface BudgetTableProps {
  /** The pack's models by kind, from the plan. */
  kinds: readonly KindRow[];
  settings: PackSettings;
  onBudget: (kind: AssetKind, triangles: number | null) => void;
}

/**
 * A triangle budget per kind, next to how many models each kind has and
 * the biggest one now. A model over its budget is decimated in Blender;
 * a rigged one never is.
 */
export default function BudgetTable({ kinds, settings, onBudget }: BudgetTableProps) {
  const budget = (kind: AssetKind) =>
    settings.budgets.find(b => b.kind === kind)?.triangles ?? null;
  const rows = kinds.map(row => {
    const b = budget(row.kind);
    const over = b !== null && row.biggest > b;
    return (
      <tr key={row.kind} className="border-t border-line">
        <td className="py-2 font-semibold">{row.label}</td>
        <td className="py-2 font-mono text-fg-dim">{row.count}</td>
        <td className="py-2">
          <div className="flex items-center gap-3">
            {b === null ? (
              <span className="w-[120px] text-fg-dim">As they are</span>
            ) : (
              <NumberField
                aria-label={`Budget for ${row.label.toLowerCase()}`}
                value={b}
                min={MIN_BUDGET}
                max={2_000_000}
                unit="tris"
                size="sm"
                format={formatCount}
                onChange={n => onBudget(row.kind, n)}
                className="w-[120px]"
              />
            )}
            <Toggle
              aria-label={`A budget for ${row.label.toLowerCase()}`}
              checked={b !== null}
              onChange={on => onBudget(row.kind, on ? Math.max(row.biggest, MIN_BUDGET) : null)}
            />
          </div>
        </td>
        <td className={`py-2 text-right font-mono ${over ? 'text-amber' : 'text-fg-dim'}`}>
          {formatCount(row.biggest)}
        </td>
      </tr>
    );
  });
  return (
    <section aria-label="Triangle budget per kind" className="flex flex-col gap-2">
      <div className="flex items-baseline gap-2">
        <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
          Triangle budget per kind
        </h3>
        <span className="text-meta text-fg-mute">
          decimated in Blender, rigged models left as they are
        </span>
      </div>
      <table className="w-full border-collapse text-left">
        <thead className="text-meta text-fg-mute">
          <tr>
            <th className="py-1 font-medium">Kind</th>
            <th className="py-1 font-medium">Count</th>
            <th className="py-1 font-medium">Budget</th>
            <th className="py-1 text-right font-medium">Biggest now</th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </section>
  );
}
