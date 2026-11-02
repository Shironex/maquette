import { Button, EmptyState } from '@maquette/ui';
import { useInboxEmpty } from './InboxEmpty.hooks';

export interface InboxEmptyProps {
  /** The inbox folder, once known. */
  dir?: string;
}

/** The empty inbox, with where a model goes to show up here. */
export default function InboxEmpty({ dir }: InboxEmptyProps) {
  const { openChecklist } = useInboxEmpty();
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-4 p-6">
      <EmptyState
        icon="tray"
        title="Nothing in the inbox"
        body={`Models you send to 3D land here. Or drop a .glb into ${dir ?? 'the inbox folder'}. I check each model before it reaches Blender.`}
      />
      <Button
        size="sm"
        icon="list"
        className="-mt-2"
        tooltip="The models you are making by hand on a provider's site"
        onClick={openChecklist}
      >
        By hand checklist
      </Button>
    </div>
  );
}
