/**
 * The pose keys turn gizmo on the rough preview: three rings on the picked
 * bone (red X, green Y, blue Z, about its parent's axes or the world's) to
 * drag round, and a dot on each body bone to pick it by, filled when the
 * change keys it. Drawn over the model, so it never hides behind it.
 */
import type { RecipeRig } from '../recipe-pose';
import type { GizmoSpace } from '../turn-gizmo';
import { useGizmoObjects, usePoseGizmo, type GizmoTurn } from './PoseGizmo.hooks';

export { RING_COLOURS } from './gizmo-look';
export type { GizmoTurn } from './PoseGizmo.hooks';

export interface PoseGizmoProps {
  rig: RecipeRig;
  bone: string;
  space: GizmoSpace;
  /** Whether the rings show; the dots show either way. */
  turning: boolean;
  /** The bones to put a dot on. */
  dots: readonly string[];
  /** The bones the change keys, whose dots are filled. */
  keyed: readonly string[];
  /** How big the model is, metres, to size the rings by. */
  size: number;
  onPick: (bone: string) => void;
  onTurn: (turn: GizmoTurn) => void;
  onDragging?: (dragging: boolean) => void;
}

export default function PoseGizmo(props: PoseGizmoProps) {
  const { rig, bone, space, dots, keyed, size } = props;
  const g = usePoseGizmo(rig, bone, space, dots, props.onTurn, props.onDragging);
  const { group, rings, balls } = useGizmoObjects(g, size, dots, bone, keyed);
  const ringNodes = rings.map((ring, i) => (
    <primitive
      key={ring.name}
      object={ring}
      onPointerDown={g.begin(i)}
      onPointerMove={g.move}
      onPointerUp={g.end}
    />
  ));
  const dotNodes = [...balls].map(([name, mesh]) => (
    <primitive
      key={name}
      object={mesh}
      onClick={(e: { stopPropagation: () => void }) => {
        e.stopPropagation();
        props.onPick(name);
      }}
    />
  ));
  return (
    <>
      {props.turning ? <primitive object={group}>{ringNodes}</primitive> : null}
      {dotNodes}
    </>
  );
}
