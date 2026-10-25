import type { InboxItem, ProjectManifest } from '@/ipc/client';
import { PrepEditor } from '@/features/common/PrepEditor';
import { useInboxPrep } from './InboxPrep.hooks';

export interface InboxPrepProps {
  item: InboxItem;
  /** Where Accept moves it, whose fix-up script a preview runs. */
  project: ProjectManifest | null;
}

/** The real size and fix-ups an inbox model gets once accepted. */
export default function InboxPrep({ item, project }: InboxPrepProps) {
  const prep = useInboxPrep(item, project);
  return (
    <section aria-label="Real size and fix-ups" className="flex flex-col gap-2">
      <PrepEditor
        value={prep.draft}
        onChange={prep.onChange}
        measured={prep.measured}
        suggestion={prep.suggestion}
        defaults={prep.defaults}
        preview={prep.preview}
        onPreview={prep.onPreview}
        previewBlocked={item.state === 'failed' ? 'This file could not be read' : null}
        skipBakeDefault
      />
      <p className="text-meta text-fg-mute">
        Saved with the new version on Accept. Every later version gets them too.
      </p>
    </section>
  );
}
