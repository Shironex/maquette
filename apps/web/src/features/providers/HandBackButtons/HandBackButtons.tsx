import { Button } from '@maquette/ui';
import type { ProviderJob } from '@/ipc/client';
import { useHandBack } from './HandBackButtons.hooks';

export interface HandBackButtonsProps {
  job: ProviderJob;
  size?: 'sm';
  /** Also offers Check Downloads. */
  downloads?: boolean;
}

/** Add model... and Check Downloads, for a send waiting for its model. */
export default function HandBackButtons({ job, size, downloads = true }: HandBackButtonsProps) {
  const { addModel, checkDownloads, busy } = useHandBack(job.id);
  return (
    <>
      <Button
        size={size}
        variant="primary"
        icon="upload-simple"
        loading={busy}
        disabled={busy}
        tooltip="Choose the GLB, or the .zip the site downloads, that you made for this send"
        onClick={() => void addModel(job.id)}
      >
        Add model...
      </Button>
      {downloads ? (
        <Button
          size={size}
          icon="download-simple"
          tooltip="Looks in your Downloads folder for the models you made"
          onClick={() => void checkDownloads()}
        >
          Check Downloads
        </Button>
      ) : null}
    </>
  );
}
