import { Button } from '@maquette/ui';
import type { InspectModel } from '@/ipc/client';
import { SectionTitle } from '../SectionTitle';
import { useLibraryPanel } from './LibraryPanel.hooks';

export interface LibraryPanelProps {
  project: string;
  model: InspectModel;
}

/** Where the selected model's asset came from in the library, an update waiting, and Add to library. */
export default function LibraryPanel({ project, model }: LibraryPanelProps) {
  const { asset, update, entry, openAdd, openUpdate } = useLibraryPanel(project, model);
  if (!asset) return null;
  const target = { project, asset: asset.id };
  const origin = asset.origin
    ? `From ${entry?.name ?? asset.origin.entry} v${asset.origin.version} in the library.`
    : 'Not in the library. Add it and any project in this workspace can use a copy.';

  return (
    <section aria-label="Library" className="flex flex-col gap-2">
      <SectionTitle aside={update ? `v${update.to} waiting` : undefined}>Library</SectionTitle>
      <p className="text-meta text-fg-mute">
        {origin}
        {update ? ` Update available: the library has v${update.to}.` : ''}
      </p>
      <div className="flex flex-wrap gap-2">
        {update ? (
          <Button
            size="sm"
            variant="primary"
            icon="arrow-up"
            onClick={() => void openUpdate(target)}
          >
            Update to v{update.to}...
          </Button>
        ) : null}
        <Button
          size="sm"
          icon="books"
          disabled={asset.versions.length === 0}
          disabledReason="No model yet"
          onClick={() => openAdd(target)}
        >
          Add to library...
        </Button>
      </div>
    </section>
  );
}
