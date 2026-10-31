import { Button, EmptyState } from '@maquette/ui';
import { RenderOptions } from '../RenderOptions';
import { RenderProblems } from '../RenderProblems';
import { RenderStage } from '../RenderStage';
import { RenderStyleList } from '../RenderStyleList';
import { TurntableSheet } from '../TurntableSheet';
import { useRenderPanel } from './RenderPanel.hooks';

/**
 * Render styles for one asset: the style and asset on the left, the picture
 * in the middle, the settings and the Render button on the right. Headless
 * Blender makes every picture; the whole project goes through the batch
 * sheet, from a Board or Models selection.
 */
export default function RenderPanel() {
  const p = useRenderPanel();
  if (!p.source || !p.asset) {
    return (
      <div className="grid flex-1 place-items-center">
        <EmptyState
          icon="image"
          title="Nothing to render yet"
          body="Render styles start from a model. Accept one from the Inbox first."
        />
      </div>
    );
  }
  if (!p.hasModel) {
    return (
      <div className="flex min-h-0 flex-1 gap-4">
        <section className="grid min-w-0 flex-1 place-items-center rounded-card border border-line bg-panel p-6">
          <EmptyState
            icon="image"
            title="Nothing to render yet"
            body={`Render styles start from a model. ${p.asset.name} has no model yet; send it to 3D or pick an asset with one.`}
            action={
              <div className="flex gap-2">
                <Button icon="cube" onClick={p.pickWithModel}>
                  Pick an asset
                </Button>
                <Button variant="primary" icon="arrow-right" onClick={p.sendTo3d}>
                  Send to 3D
                </Button>
              </div>
            }
          />
        </section>
        <RenderProblems
          className="w-[440px]"
          problems={p.problems}
          source={p.source}
          asset={p.asset}
          name={p.problemName}
        />
      </div>
    );
  }
  return (
    <div className="flex min-h-0 flex-1 gap-4">
      <RenderStyleList assets={p.assets} asset={p.asset} onPick={p.pick} />
      <RenderStage />
      <div className="flex min-h-0 w-[340px] flex-none flex-col gap-3 overflow-y-auto">
        <RenderProblems
          problems={p.problems}
          source={p.source}
          asset={p.asset}
          name={p.problemName}
        />
        <RenderOptions source={p.source} assetId={p.asset.id} />
      </div>
      <TurntableSheet source={p.source} assetId={p.asset.id} name={p.asset.name} />
    </div>
  );
}
