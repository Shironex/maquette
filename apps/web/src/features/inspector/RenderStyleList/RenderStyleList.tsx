import { Select, cn } from '@maquette/ui';
import type { Asset } from '@/ipc/client';
import { STYLES } from '@/state/renders';
import { useRenderStyleList } from './RenderStyleList.hooks';

export interface RenderStyleListProps {
  assets: readonly Asset[];
  asset: Asset;
  onPick: (key: string) => void;
}

/** The six styles and the asset they render. */
export default function RenderStyleList({ assets, asset, onPick }: RenderStyleListProps) {
  const { style, setStyle } = useRenderStyleList();
  const options = assets.map(a => {
    const last = a.versions[a.versions.length - 1];
    return {
      value: a.id,
      label: last ? `${a.name} · v${last.version}` : `${a.name} · no model`,
    };
  });
  const rows = STYLES.map(s => (
    <button
      key={s.id}
      type="button"
      aria-pressed={style === s.id}
      onClick={() => setStyle(s.id)}
      className={cn(
        'flex flex-col rounded-control border px-2.5 py-2 text-left transition-colors duration-120',
        style === s.id ? 'border-clay-text/50 bg-clay-soft' : 'border-transparent hover:bg-hover'
      )}
    >
      <span className="font-medium">{s.label}</span>
      <span className="text-meta text-fg-mute">{s.line}</span>
    </button>
  ));
  return (
    <aside
      aria-label="Styles"
      className="flex w-[232px] flex-none flex-col gap-1 overflow-y-auto rounded-card border border-line bg-panel p-3"
    >
      <h3 className="mb-1 text-panel-title">Styles</h3>
      {rows}
      <div className="mt-auto flex flex-col gap-1.5 border-t border-line pt-3">
        <Select label="Asset" value={asset.id} options={options} onChange={onPick} />
        <p className="text-meta text-fg-mute">
          Or the whole project: select assets on the Board, then Render, for one image per asset and
          a contact sheet.
        </p>
      </div>
    </aside>
  );
}
