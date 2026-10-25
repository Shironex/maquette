import { IconButton } from '@maquette/ui';

export interface RefChipProps {
  url: string | null;
  name: string;
  onRemove: () => void;
}

/** One reference image of the brief, with its Remove button. */
export default function RefChip({ url, name, onRemove }: RefChipProps) {
  return (
    <figure
      className="group relative m-0 h-[62px] overflow-hidden rounded-card bg-raised"
      title={name}
    >
      {url ? (
        <img src={url} alt={name} className="block size-full object-cover" />
      ) : (
        <span className="grid size-full place-items-center px-1 text-center text-meta text-fg-mute">
          {name}
        </span>
      )}
      <IconButton
        icon="x"
        label={`Remove ${name}`}
        size="sm"
        variant="secondary"
        onClick={onRemove}
        className="absolute top-1 right-1 size-6 opacity-0 group-focus-within:opacity-100 group-hover:opacity-100"
      />
    </figure>
  );
}
