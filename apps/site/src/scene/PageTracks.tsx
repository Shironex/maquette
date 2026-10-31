/** The timeline's render style and hero text tracks, played on the canvas and the page. */
import { usePageTracks } from './PageTracks.hooks';

export interface PageTracksProps {
  /** Tour progress in stops. */
  progress: React.RefObject<number>;
}

export function PageTracks({ progress }: PageTracksProps) {
  usePageTracks(progress);
  return null;
}
