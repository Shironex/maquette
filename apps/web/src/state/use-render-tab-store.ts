/**
 * The Inspector's Render tab: the asset and style on show, their settings
 * and the pictures the core answers with. A preview is cached by the core,
 * so going back to settings seen before answers at once; an older answer
 * is dropped when a newer question went out.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type PackSource,
  type RenderPreview,
  type RenderSettings,
  type RenderShot,
  type RenderStyle,
} from '@/ipc/client';
import { RENDER_DEFAULTS, isLook } from './renders';

/** The stage's picture and the eight pictures of the strip. */
const STAGE = 520;
const TILE = 160;
const LOOP_SAMPLE = 480;

function shotsOf(style: RenderStyle, settings: RenderSettings): RenderShot[] {
  const strip: RenderShot[] = [
    { style: 'blueprint', turn: 0, size: TILE },
    { style: 'toon', turn: 0, size: TILE },
    { style: 'clay', turn: 0, size: TILE },
    { style: 'shaded', turn: 0, size: TILE },
    { style: 'clay', turn: 90, size: TILE },
    { style: 'clay', turn: 180, size: TILE },
    { style: 'clay', turn: 270, size: TILE },
  ];
  const stage: RenderShot = isLook(style)
    ? { style, turn: 0, size: STAGE }
    : { style, turn: 0, size: settings.width ?? LOOP_SAMPLE };
  return [stage, ...strip];
}

interface RenderTabStore {
  picked: string | null;
  style: RenderStyle;
  settings: Required<RenderSettings>;
  preview: RenderPreview | null;
  loading: boolean;
  error: string | null;
  turntableOpen: boolean;
  pick: (key: string) => void;
  setStyle: (style: RenderStyle) => void;
  change: (change: Partial<RenderSettings>) => void;
  reset: () => void;
  load: (source: PackSource, key: string) => Promise<void>;
  setTurntable: (open: boolean) => void;
}

let asked = 0;

export const useRenderTabStore = create<RenderTabStore>((set, get) => ({
  picked: null,
  style: 'blueprint',
  settings: RENDER_DEFAULTS,
  preview: null,
  loading: false,
  error: null,
  turntableOpen: false,

  pick: key => set({ picked: key, preview: null, error: null }),
  setStyle: style =>
    set(s => ({
      style,
      settings: isLook(style) ? { ...s.settings, look: style, paperColor: null } : s.settings,
      turntableOpen: style === 'turntable' ? true : s.turntableOpen,
    })),
  change: change => set(s => ({ settings: { ...s.settings, ...change } })),
  reset: () => set({ settings: { ...RENDER_DEFAULTS, look: get().settings.look } }),
  setTurntable: turntableOpen => set({ turntableOpen }),

  load: async (source, key) => {
    const { style, settings } = get();
    asked += 1;
    const mine = asked;
    set({ loading: true, error: null });
    try {
      const preview = await commands.renderPreview(source, key, settings, shotsOf(style, settings));
      if (mine === asked) set({ preview, loading: false });
    } catch (error) {
      if (mine === asked) set({ error: errorMessage(error), loading: false });
    }
  },
}));
