import { Button, cn } from '@maquette/ui';
import { useShipTable } from './ShipTable.hooks';

/** Current ship against this run: what the site holds now and what would ship next. */
export default function ShipTable() {
  const { rows, loaded, note, compare } = useShipTable();
  const body = rows.map(r => (
    <tr key={r.label} className="border-b border-line">
      <td className="h-[30px] px-2 whitespace-nowrap text-fg">{r.label}</td>
      <td className="px-2 text-right font-mono text-[11.5px] whitespace-nowrap text-fg-dim">
        {r.ship}
      </td>
      <td className="px-2 text-right font-mono text-[11.5px] whitespace-nowrap text-fg">{r.run}</td>
      <td
        className={cn(
          'px-2 text-right font-mono text-[11.5px] whitespace-nowrap',
          r.better === true ? 'text-sage' : r.better === false ? 'text-danger' : 'text-fg-mute'
        )}
      >
        {r.change}
      </td>
    </tr>
  ));
  return (
    <section aria-labelledby="page-ship" className="flex flex-col gap-2.5">
      <div className="flex items-center justify-between gap-2">
        <h2
          id="page-ship"
          className="m-0 text-caption font-medium tracking-[0.04em] text-fg-mute uppercase"
        >
          Current ship vs this run
        </h2>
        <Button size="sm" icon="columns" onClick={compare}>
          Compare
        </Button>
      </div>
      {rows.length ? (
        <table className="w-full table-fixed border-collapse text-[12px]">
          <colgroup>
            <col />
            <col className="w-[62px]" />
            <col className="w-[76px]" />
            <col className="w-[70px]" />
          </colgroup>
          <thead>
            <tr className="border-b border-line text-[11px] tracking-[0.04em] text-fg-mute uppercase">
              <th className="px-2 pb-1.5 text-left font-medium">
                <span className="sr-only">Number</span>
              </th>
              <th className="px-2 pb-1.5 text-right font-medium">Ship</th>
              <th className="px-2 pb-1.5 text-right font-medium whitespace-nowrap">This run</th>
              <th className="px-2 pb-1.5 text-right font-medium">Change</th>
            </tr>
          </thead>
          <tbody>{body}</tbody>
        </table>
      ) : null}
      {note || !loaded ? (
        <p className="m-0 text-meta text-fg-mute">{note ?? 'Reading the models'}</p>
      ) : null}
    </section>
  );
}
