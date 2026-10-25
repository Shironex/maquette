import { EmptyState } from '@maquette/ui';

export interface InboxEmptyProps {
  /** The inbox folder, once known. */
  dir?: string;
}

/** The empty inbox, with where a model goes to show up here. */
export default function InboxEmpty({ dir }: InboxEmptyProps) {
  return (
    <div className="grid flex-1 place-items-center p-6">
      <EmptyState
        icon="tray"
        title="Nothing in the inbox"
        body={`Drop a .glb from Tripo into ${dir ?? 'the inbox folder'}. I check each model before it reaches Blender.`}
      />
    </div>
  );
}
