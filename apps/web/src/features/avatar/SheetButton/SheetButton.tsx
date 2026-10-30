import { Icon } from '@maquette/ui';

export interface SheetButtonProps {
  src: string | null;
  label: string;
  onOpen: () => void;
}

/** A before or after sheet as a thumbnail that opens it up close. */
export default function SheetButton({ src, label, onOpen }: SheetButtonProps) {
  return (
    <figure className="m-0 flex min-w-0 flex-1 flex-col gap-1.5">
      <figcaption className="text-meta font-medium text-fg-dim">{label}</figcaption>
      <button
        type="button"
        onClick={onOpen}
        title={`Opens the ${label.toLowerCase()} sheet up close`}
        className="group relative cursor-zoom-in overflow-hidden rounded-[8px] border border-line bg-studio"
      >
        {src ? (
          <img src={src} alt={`The ${label.toLowerCase()} sheet`} className="block w-full" />
        ) : (
          <span className="block aspect-square" />
        )}
        <span className="absolute right-2 bottom-2 grid size-7 place-items-center rounded-[7px] bg-panel/90 text-fg-dim opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100">
          <Icon name="magnifying-glass" size={15} />
        </span>
      </button>
    </figure>
  );
}
