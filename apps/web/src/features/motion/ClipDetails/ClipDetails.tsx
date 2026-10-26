import { KeyValueList } from '@maquette/ui';
import { SectionLabel } from '@/features/common/SectionLabel';
import { useClipDetails } from './ClipDetails.hooks';

/** The selected clip's numbers. Name, rate and compression come from the file. */
export default function ClipDetails() {
  const details = useClipDetails();
  if (!details) return null;
  return (
    <section aria-label="Clip details" className="flex flex-col gap-3 border-t border-line pt-3.5">
      <div className="flex items-center justify-between gap-2">
        <span className="min-w-0 truncate" title={details.clip.name}>
          <SectionLabel className="truncate">{details.clip.name}</SectionLabel>
        </span>
        <span className="flex-none text-meta text-fg-mute">{details.status}</span>
      </div>
      <KeyValueList items={details.items} />
      <p className="text-meta text-fg-mute">
        Name, rate and keys come from the model file. In and Out on the timeline loop the preview
        only; the clip ships whole.
      </p>
    </section>
  );
}
