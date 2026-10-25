import { useEffect, useState } from 'react';
import { commands, errorMessage, type ImportScan } from '@/ipc/client';

/** Scans the folder to import and turns the result into the field note. */
export function useImportPicker(folder: string) {
  const [scan, setScan] = useState<ImportScan | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setScan(null);
    setError(null);
    if (!folder.trim()) return;
    commands
      .projectScanImport(folder)
      .then(result => live && setScan(result))
      .catch((e: unknown) => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
  }, [folder]);

  const latest = scan?.blendFiles[0];
  return error
    ? { tone: 'missing' as const, text: error }
    : scan
      ? {
          tone: scan.blendFiles.length ? ('found' as const) : ('warning' as const),
          text: scan.blendFiles.length
            ? `${scan.blendFiles.length} .blend files, latest ${latest?.file ?? ''}. ${scan.toolScripts.length} tools/ scripts.`
            : 'No .blend files found. I will import the folder as it is.',
        }
      : undefined;
}
