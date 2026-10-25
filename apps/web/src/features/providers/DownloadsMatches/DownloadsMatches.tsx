/**
 * Check Downloads: the new GLB and zip files in the Downloads folder,
 * each with the waiting send it most likely is. Nothing is taken until
 * you press Use on a row, and then the file is copied, never moved.
 */
import { DownloadsDialog } from './DownloadsDialog';
import { useDownloadsOpen } from './DownloadsMatches.hooks';

export default function DownloadsMatches() {
  const { open, close } = useDownloadsOpen();
  if (!open) return null;
  return <DownloadsDialog onClose={close} />;
}
