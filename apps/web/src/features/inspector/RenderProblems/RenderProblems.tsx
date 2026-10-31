import { cn } from '@maquette/ui';
import type { Asset, PackSource, ProblemKind } from '@/ipc/client';
import { RenderProblemCard, type ProblemAction } from '@/features/common/RenderProblemCard';
import { useRenderStore } from '@/state/use-render-store';
import { useRenderTabStore } from '@/state/use-render-tab-store';
import type { FoundProblem } from '../RenderPanel/RenderPanel.hooks';

export interface RenderProblemsProps {
  problems: readonly FoundProblem[];
  source: PackSource;
  asset: Asset;
  /** The model the problems are about. */
  name: string;
  className?: string;
}

/** What went wrong in the asset's last render, each with what to do next. */
export default function RenderProblems({
  problems,
  source,
  asset,
  name,
  className,
}: RenderProblemsProps) {
  if (problems.length === 0) return null;
  const { settings } = useRenderTabStore.getState();
  const store = useRenderStore.getState;
  const again = (found: FoundProblem, change: Partial<typeof settings>) =>
    void store().renderOne(source, asset.id, found.style, { ...settings, ...change });
  const actions = (found: FoundProblem): ProblemAction[] => {
    const kind: ProblemKind = found.problem.kind;
    if (kind === 'missing-tool') {
      return [{ label: 'Open Toolchain', icon: 'wrench', onClick: () => store().openToolchain() }];
    }
    if (kind === 'over-target') {
      return [
        {
          label: 'Crop to the model',
          icon: 'resize',
          onClick: () => again(found, { format: 'gif', tightCrop: true }),
        },
        {
          label: 'Save as WebP',
          icon: 'image',
          onClick: () => again(found, { format: 'webp' }),
        },
      ];
    }
    if (kind === 'out-of-memory') {
      return [
        {
          label: 'Try at 2048 px',
          icon: 'arrow-clockwise',
          onClick: () => again(found, { size: 2048 }),
        },
      ];
    }
    return [];
  };
  const cards = problems.map(found => (
    <RenderProblemCard
      key={`${found.problem.kind}-${found.style}`}
      problem={found.problem}
      style={found.style}
      bytes={found.bytes}
      name={name}
      actions={actions(found)}
    />
  ));
  return (
    <section
      aria-label="When a render goes wrong"
      className={cn(
        'flex flex-none flex-col gap-2.5 self-start rounded-card border border-line bg-panel p-3.5',
        className
      )}
    >
      <div>
        <h3 className="text-panel-title">When a render goes wrong</h3>
        <p className="text-meta text-fg-mute">Each says what to do next.</p>
      </div>
      {cards}
    </section>
  );
}
