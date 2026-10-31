import { useRenderTabStore } from '@/state/use-render-tab-store';

/** The style on show and how to change it. */
export function useRenderStyleList() {
  const style = useRenderTabStore(s => s.style);
  const setStyle = useRenderTabStore(s => s.setStyle);
  return { style, setStyle };
}
