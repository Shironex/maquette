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
        onClick={() => void addModel(job.id)}
      >
        Add model...
      </Button>
      {downloads ? (
        <Button size={size} icon="download-simple" onClick={() => void checkDownloads()}>
          Check Downloads
        </Button>
      ) : null}
    </>
  );
}
