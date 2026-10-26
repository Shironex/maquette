import { AddToLibraryDialog } from '../AddToLibraryDialog';
import { LibraryUpdateDialog } from '../LibraryUpdateDialog';
import { useLibraryDialogs } from './LibraryDialogs.hooks';

/** Add to library and Update from library, wherever they were opened from. */
export default function LibraryDialogs() {
  const { adding, updating } = useLibraryDialogs();
  const addKey = adding === 'pick' ? 'pick' : `${adding?.project}/${adding?.asset}`;
  return (
    <>
      {adding ? <AddToLibraryDialog key={addKey} target={adding} /> : null}
      {updating ? <LibraryUpdateDialog key={`${updating.project}/${updating.asset}`} /> : null}
    </>
  );
}
