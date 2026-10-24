import { EmptyState, TopBar, type IconName } from '@maquette/ui';
import { TOOL_VIEWS, type ViewId } from '@/features/shell/views';

const COPY: Partial<Record<ViewId, { icon: IconName; title: string; body: string }>> = {
  inbox: {
    icon: 'tray',
    title: 'Nothing in the inbox',
    body: 'Tripo downloads land here and I check each model before it reaches Blender. The inbox arrives in a later version.',
  },
  lab: {
    icon: 'flask',
    title: 'The Concept lab is on its way',
    body: 'Generate a few directions with Codex, then star the ones worth modelling. The lab arrives in a later version.',
  },
  pipeline: {
    icon: 'flow-arrow',
    title: 'No pipeline runs yet',
    body: 'Each run shows its steps here, from the Blender build to Compress, with logs and cached steps. The runner arrives in a later version.',
  },
  inspector: {
    icon: 'cube-focus',
    title: 'Nothing to inspect yet',
    body: 'Pick a model on the Board to check its triangles, textures and budget in 3D. The viewer arrives in a later version.',
  },
};

export function PlaceholderView({ view }: { view: ViewId }) {
  const copy = COPY[view];
  const label = TOOL_VIEWS.find(v => v.id === view)?.label ?? 'View';
  return (
    <>
      <TopBar lead={<h1 className="text-panel-title">{label}</h1>} />
      <main className="grid min-h-0 flex-1 place-items-center overflow-y-auto p-6">
        {copy ? <EmptyState icon={copy.icon} title={copy.title} body={copy.body} /> : null}
      </main>
    </>
  );
}
