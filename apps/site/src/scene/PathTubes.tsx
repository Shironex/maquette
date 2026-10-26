/** Maquette's paths, lighting up as the page scrolls (see ./PathTubes.hooks.ts). */
import { usePathTubes } from './PathTubes.hooks';

export interface PathTubesProps {
  /** Tour progress in stops. */
  progress: React.RefObject<number>;
}

export function PathTubes({ progress }: PathTubesProps) {
  const group = usePathTubes(progress);
  return <primitive object={group} />;
}
