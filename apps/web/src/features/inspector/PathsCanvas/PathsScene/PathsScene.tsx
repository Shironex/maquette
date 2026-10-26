import { usePathsScene } from './PathsScene.hooks';
import type { PathsSceneProps } from './PathsScene.types';

/** Everything inside the paths canvas; it draws through three directly, so it renders nothing. */
export default function PathsScene(props: PathsSceneProps) {
  usePathsScene(props);
  return null;
}
