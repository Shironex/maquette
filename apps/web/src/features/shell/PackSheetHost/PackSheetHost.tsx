import { Suspense, lazy } from 'react';
import PackSheetBoundary from './PackSheetBoundary';
import { usePackSheetHost } from './PackSheetHost.hooks';

// About 8 kB gzipped that most sessions never open, so it stays out of the first chunk.
const PackSheet = lazy(async () => ({
  default: (await import('@/features/library/PackSheet')).PackSheet,
}));

/** The pack sheet, over whichever view asked for it, loaded the first time it opens. */
export default function PackSheetHost() {
  const { open, close } = usePackSheetHost();
  if (!open) return null;
  return (
    <PackSheetBoundary onClose={close}>
      <Suspense fallback={null}>
        <PackSheet />
      </Suspense>
    </PackSheetBoundary>
  );
}
