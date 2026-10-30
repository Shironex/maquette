import { useQuickView } from './QuickView.hooks';
import QuickViewDialog from './QuickViewDialog';

/** The 3D quick view, wherever it was opened from: the Board's card menu or the Library. */
export default function QuickView() {
  const { target, close } = useQuickView();
  return target ? <QuickViewDialog key={target.file} target={target} onClose={close} /> : null;
}
