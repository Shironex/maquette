import { useFrame, useThree } from '@react-three/fiber';
import { useMemo } from 'react';
import * as THREE from 'three';
import { BENCH_TOP } from './manifest';
import { presence } from './progress';

const ANCHORS: Record<string, readonly [number, number, number]> = {
  concept: [-0.42, BENCH_TOP + 0.01, 0.27],
  model: [-0.17, BENCH_TOP + 0.04, 0.07],
  web: [0.5, BENCH_TOP + 0.01, 0.13],
};

/** Projects each `[data-callout]` label's anchor to the screen every frame the hero shows. */
export function useCallouts(progress: React.RefObject<number>, reducedMotion: boolean) {
  const size = useThree(state => state.size);
  const labels = useMemo(
    () => Array.from(document.querySelectorAll<HTMLElement>('[data-callout]')),
    []
  );
  const point = useMemo(() => new THREE.Vector3(), []);

  useFrame(({ camera }) => {
    if (presence(progress.current, 0, reducedMotion) === 0) return;
    for (const label of labels) {
      const anchor = ANCHORS[label.dataset.callout ?? ''];
      if (!anchor) continue;
      point.set(...anchor).project(camera);
      const x = ((point.x + 1) / 2) * size.width;
      const y = ((1 - point.y) / 2) * size.height;
      label.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px)`;
      label.dataset.placed = 'true';
    }
  });
}
