import type { InspectModel } from '@/ipc/client';
import { PrepEditor } from '@/features/common/PrepEditor';
import { usePrepPanel } from './PrepPanel.hooks';

export interface PrepPanelProps {
  project: string;
  model: InspectModel;
}

/** The selected model's real size and fix-ups, in the Inspector's model details. */
export default function PrepPanel({ project, model }: PrepPanelProps) {
  const { asset, found, failed, preview, onChange, onPreview, applied } = usePrepPanel(
    project,
    model
  );
  if (!asset) return null;
  return (
    <section aria-label="Real size and fix-ups" className="flex flex-col gap-2.5">
      {failed ? <p className="text-meta text-danger">{failed}</p> : null}
      {found ? (
        <PrepEditor
          value={found.entry}
          onChange={onChange}
          measured={found.measured}
          fixed={found.fixed}
          suggestion={found.suggestion}
          defaults={found.defaults}
          preview={preview}
          onPreview={onPreview}
          previewBlocked={found.version === null ? 'The asset has no model yet' : null}
        />
      ) : failed ? null : (
        <p className="text-fg-mute">Reading the saved real size and fix-ups.</p>
      )}
      <p className="text-meta text-fg-mute">{applied}</p>
    </section>
  );
}
