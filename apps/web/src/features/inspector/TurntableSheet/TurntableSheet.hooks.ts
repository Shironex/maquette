import { useEffect, useState } from 'react';
import type { PackSource, RenderSettings } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { FPS, TARGET_BYTES } from '@/state/renders';
import { isUsable } from '@/state/toolchain';
import { useAppStore } from '@/state/use-app-store';
import { useRenderStore } from '@/state/use-render-store';
import { useRenderTabStore } from '@/state/use-render-tab-store';

const QUIET_MS = 350;
/** The sample's frames play slower than the file: eight of a whole turn. */
const SAMPLE_MS = 180;

/** The turntable sheet: its settings, the sample's frames, the crop box and the size. */
export function useTurntableSheet(source: PackSource, assetId: string) {
  const open = useRenderTabStore(s => s.turntableOpen);
  const setOpen = useRenderTabStore(s => s.setTurntable);
  const settings = useRenderTabStore(s => s.settings);
  const change = useRenderTabStore(s => s.change);
  const preview = useRenderTabStore(s => s.preview);
  const reports = useAppStore(s => s.tools.reports);
  const [frame, setFrame] = useState(0);
  const [playing, setPlaying] = useState(false);
  const shot = preview?.shots.find(s => s.style === 'turntable');
  const frames = shot?.frames ?? [];

  useEffect(() => {
    if (!open || !playing || frames.length === 0) return;
    const timer = setInterval(() => setFrame(f => (f + 1) % frames.length), SAMPLE_MS);
    return () => clearInterval(timer);
  }, [open, playing, frames.length]);

  useEffect(() => {
    if (!open) return;
    const timer = setTimeout(
      () => void useRenderTabStore.getState().load(source, assetId),
      QUIET_MS
    );
    return () => clearTimeout(timer);
  }, [open, source, assetId, settings]);

  const ffmpeg = reports.find(r => r.id === 'ffmpeg');
  const needsFfmpeg = settings.format !== 'webp' && ffmpeg !== undefined && !isUsable(ffmpeg);
  const total = settings.length * FPS;
  const shown = Math.min(frame, Math.max(frames.length - 1, 0));
  const frameNumber = frames.length > 0 ? Math.round((shown * total) / frames.length) + 1 : 1;
  const close = () => {
    setPlaying(false);
    setOpen(false);
    useRenderTabStore.setState(s => (s.style === 'turntable' ? { style: s.settings.look } : s));
  };
  return {
    open,
    settings,
    change: (next: Partial<RenderSettings>) => change(next),
    moving: shot?.moving ?? null,
    estimate: shot?.estimateBytes ?? null,
    over: (shot?.estimateBytes ?? 0) > TARGET_BYTES,
    frames: frames.map(f => fileUrl(f)),
    shown,
    pick: (i: number) => {
      setPlaying(false);
      setFrame(i);
    },
    playing,
    togglePlay: () => setPlaying(p => !p),
    frameLine: `Frame ${frameNumber} of ${total} · ${((frameNumber - 1) / FPS).toFixed(2)} s`,
    total,
    needsFfmpeg,
    folder: preview?.folder ?? '',
    dir: preview?.dir ?? null,
    close,
    render: () => {
      void useRenderStore.getState().renderOne(source, assetId, 'turntable', settings);
      close();
    },
    addToBatch: () => {
      void useRenderStore.getState().addToBatch(source, assetId, 'turntable', settings);
      close();
    },
    openToolchain: () => {
      close();
      useRenderStore.getState().openToolchain();
    },
  };
}
