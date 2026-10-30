import { useRef, useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { reducedMotion } from '@/lib/motion';
import { useTopEscape } from '@/lib/useTopEscape';
import type { PreviewMode } from '@/features/viewer/ModelPreview/ModelPreview.types';
import { MODES } from '@/features/viewer/previewModes';
import { useQuickViewStore } from '@/state/use-quick-view-store';
import type { QuickViewDialogProps } from './QuickView.types';

/** The model on show, if the quick view is open, and how to close it. */
export function useQuickView() {
  const target = useQuickViewStore(s => s.target);
  const close = useQuickViewStore(s => s.close);
  return { target, close };
}

/** The modes a quick view offers: the Inbox's, without Parts, which needs a split preview. */
const QUICK_MODES = MODES.filter(m => m.value !== 'parts');

/**
 * The quick view's viewport: its mode, the turntable and what the preview
 * reported, with Escape, the focus trap and focus back to the opener.
 */
export function useQuickViewDialog({ target, onClose }: QuickViewDialogProps) {
  const ref = useRef<HTMLDivElement>(null);
  useTopEscape(ref, onClose);
  const openInspector = useQuickViewStore(s => s.openInspector);
  const [mode, setMode] = useState<PreviewMode>('shaded');
  // It spins until stopped, so it starts still when the system asks for less motion.
  const [turning, setTurning] = useState(() => !reducedMotion());
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loadedBones, setLoadedBones] = useState(0);
  const url = fileUrl(target.file);
  const rigged = (target.stats?.joints ?? 0) > 0 || loadedBones > 0;
  const showing = Boolean(url) && !loadError;
  return {
    ref,
    url,
    mode,
    setMode,
    modes: QUICK_MODES.filter(m => m.value !== 'bones' || rigged),
    modeLabel: QUICK_MODES.find(m => m.value === mode)?.label ?? 'Shaded',
    turning,
    toggleTurning: () => setTurning(on => !on),
    loadError,
    setLoadError,
    onLoaded: (info: { bones: number }) => setLoadedBones(info.bones),
    showing,
    inspect: target.inspect ? () => target.inspect && openInspector(target.inspect) : null,
  };
}
