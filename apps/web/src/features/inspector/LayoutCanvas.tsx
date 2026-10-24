/**
 * The Layout editor's 3D view: every model of the project where the layout
 * puts it, the desk, a ground grid and the camera stops. A click selects a
 * model or a stop and three's transform gizmo moves, turns or scales it;
 * a drag elsewhere orbits. Looking through a stop flies the camera there.
 * Loaded lazily, with three.js.
 */
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { TransformControls } from 'three/examples/jsm/controls/TransformControls.js';
import type { LayoutModel } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { FLIGHT_MS, flight, type CameraPose, type Vec3 } from '@/state/inspector';
import {
  DESK_THICKNESS,
  SNAP,
  placementOf,
  type PlainLayout,
  type PlainPlacement,
  type PlainStop,
  type Tool,
} from '@/state/layout';
import type { Selection } from '@/state/layout-store';
import { BACKDROP, CLAY, castShadows, frameCamera, gltfLoader } from '@/features/viewer/studio';

export interface LayoutCanvasProps {
  models: LayoutModel[];
  layout: PlainLayout;
  selection: Selection;
  tool: Tool;
  snap: boolean;
  grid: boolean;
  preview: { id: string; at: number } | null;
  /** A counter: each change frames every model again. */
  frame: number;
  onSelect: (selection: Selection) => void;
  onBeginDrag: () => void;
  onDragModel: (id: string, placement: PlainPlacement) => void;
  onDragStop: (id: string, position: Vec3) => void;
  /** Where the camera stands after the person moved it, or after a flight. */
  onPose: (pose: CameraPose) => void;
  /** The camera was moved by hand, so it no longer looks through a stop. */
  onUserMove: () => void;
  onError: (message: string) => void;
  onReady: () => void;
  className?: string;
}

const LINE = '#4d4a44';
const MUTE = '#9a9488';
const SAGE = '#8fa58c';
const DESK = '#6e5a49';
/** How long a stop's frustum is drawn, as a share of the scene's radius. */
const FRUSTUM = 0.07;
const ASPECT = 16 / 9;
const deg = THREE.MathUtils.degToRad;

function disposeTree(object: THREE.Object3D) {
  object.traverse(o => {
    const mesh = o as THREE.Mesh;
    if (!mesh.geometry) return;
    mesh.geometry.dispose();
    const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    materials.forEach(m => m?.dispose());
  });
}

function applyPlacement(holder: THREE.Object3D, p: PlainPlacement) {
  holder.position.set(...p.position);
  holder.rotation.set(deg(p.rotation[0]), deg(p.rotation[1]), deg(p.rotation[2]), 'XYZ');
  holder.scale.setScalar(p.scale);
}

function placementFrom(holder: THREE.Object3D): PlainPlacement {
  const r = holder.rotation;
  return {
    position: holder.position.toArray() as Vec3,
    rotation: [r.x, r.y, r.z].map(THREE.MathUtils.radToDeg) as Vec3,
    scale: holder.scale.x,
  };
}

/** Loads every model once; each sits in a holder the layout moves. */
function useHolders(
  models: LayoutModel[],
  onError: (message: string) => void
): Map<string, THREE.Group> | null {
  const gl = useThree(s => s.gl);
  const [holders, setHolders] = useState<Map<string, THREE.Group> | null>(null);
  const key = models.map(m => `${m.asset}|${m.file}`).join('\n');
  useEffect(() => {
    let alive = true;
    const made = new Map<string, THREE.Group>();
    const loader = gltfLoader(gl);
    Promise.allSettled(
      models.map(async model => {
        const url = fileUrl(model.file);
        if (!url) throw new Error(`${model.name} has no file to show.`);
        const gltf = await loader.loadAsync(url);
        const holder = new THREE.Group();
        holder.name = model.asset;
        holder.userData.asset = model.asset;
        holder.add(gltf.scene);
        castShadows(holder);
        holder.traverse(o => {
          if ((o as THREE.Mesh).isMesh) o.receiveShadow = true;
        });
        made.set(model.asset, holder);
      })
    ).then(results => {
      if (!alive) return;
      const failed = results.filter(r => r.status === 'rejected');
      if (failed.length > 0) {
        onError(
          failed.length === results.length
            ? 'The models did not load.'
            : `${failed.length} of ${results.length} models did not load.`
        );
      }
      setHolders(made);
    });
    return () => {
      alive = false;
      for (const holder of made.values()) disposeTree(holder);
    };
    // `key` stands for `models`; onError is a callback prop.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, gl]);
  return holders;
}

/** A stop drawn as a small camera: a frustum toward its target and a dot there. */
function stopGlyph(stop: PlainStop, color: string, first: boolean, span: number): THREE.Group {
  const length = Math.max(0.06, span * FRUSTUM);
  const group = new THREE.Group();
  group.userData.stop = stop.id;
  const from = new THREE.Vector3(...stop.position);
  const to = new THREE.Vector3(...stop.target);
  const forward = to.clone().sub(from).normalize();
  const right = new THREE.Vector3().crossVectors(forward, new THREE.Vector3(0, 1, 0));
  if (right.lengthSq() < 1e-6) right.set(1, 0, 0);
  right.normalize();
  const up = new THREE.Vector3().crossVectors(right, forward).normalize();
  const h = Math.tan(deg(stop.fov) / 2) * length;
  const w = h * ASPECT;
  const center = forward.clone().multiplyScalar(length);
  const corners = [
    [-1, -1],
    [1, -1],
    [1, 1],
    [-1, 1],
  ].map(([x, y]) =>
    center
      .clone()
      .addScaledVector(right, x! * w)
      .addScaledVector(up, y! * h)
  );
  const points: THREE.Vector3[] = [];
  for (let i = 0; i < 4; i++) {
    points.push(new THREE.Vector3(), corners[i]!, corners[i]!, corners[(i + 1) % 4]!);
  }
  // The little roof marks which way is up.
  const roof = center.clone().addScaledVector(up, h * 1.6);
  points.push(corners[2]!, roof, roof, corners[3]!);
  const frustum = new THREE.LineSegments(
    new THREE.BufferGeometry().setFromPoints(points),
    new THREE.LineBasicMaterial({ color, depthTest: false, transparent: true })
  );
  frustum.renderOrder = 3;
  const toward = to.clone().sub(from);
  const sight = new THREE.Line(
    new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(), toward]),
    new THREE.LineDashedMaterial({
      color,
      dashSize: length * 0.16,
      gapSize: length * 0.12,
      depthTest: false,
      transparent: true,
      opacity: 0.6,
    })
  );
  sight.computeLineDistances();
  sight.renderOrder = 3;
  const dot = new THREE.Mesh(
    new THREE.SphereGeometry(length * (first ? 0.08 : 0.06), 12, 8),
    new THREE.MeshBasicMaterial({ color, depthTest: false, transparent: true })
  );
  dot.position.copy(toward);
  dot.renderOrder = 3;
  // An invisible box at the camera to click and to hang the gizmo on.
  const body = new THREE.Mesh(
    new THREE.BoxGeometry(length * 0.45, length * 0.45, length * 0.45),
    new THREE.MeshBasicMaterial({ visible: false })
  );
  body.userData.stop = stop.id;
  body.name = 'stop-body';
  // The group stands at the camera, so the gizmo moves the whole glyph.
  group.position.copy(from);
  group.add(frustum, sight, dot, body);
  return group;
}

function poseOf(camera: THREE.PerspectiveCamera, target: THREE.Vector3): CameraPose {
  return {
    position: camera.position.toArray() as Vec3,
    target: target.toArray() as Vec3,
    fov: camera.fov,
  };
}

/**
 * The vertical field of view that shows a 16:9 poster's whole frame: the
 * stop's own when the view is at least as wide, else wider, so the frame
 * fits the width (the letterbox marks it).
 */
function fitFov(fov: number, aspect: number): number {
  if (!(aspect > 0) || aspect >= ASPECT) return fov;
  const half = Math.atan((Math.tan(deg(fov) / 2) * ASPECT) / aspect);
  return THREE.MathUtils.radToDeg(half * 2);
}

function setPose(camera: THREE.PerspectiveCamera, controls: OrbitControls, pose: CameraPose) {
  camera.position.set(...pose.position);
  controls.target.set(...pose.target);
  if (camera.fov !== pose.fov) {
    camera.fov = pose.fov;
    camera.updateProjectionMatrix();
  }
  camera.lookAt(controls.target);
}

function Editor(props: Omit<LayoutCanvasProps, 'className'>) {
  const { gl, scene, size } = useThree();
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const holders = useHolders(props.models, props.onError);
  const live = useRef(props);
  useLayoutEffect(() => {
    live.current = props;
  });
  const orbit = useMemo(() => new OrbitControls(camera, gl.domElement), [camera, gl]);
  const transform = useMemo(() => new TransformControls(camera, gl.domElement), [camera, gl]);
  const world = useMemo(() => new THREE.Group(), []);
  const stopsGroup = useMemo(() => new THREE.Group(), []);
  const box = useMemo(() => new THREE.Box3Helper(new THREE.Box3(), CLAY), []);
  const raycaster = useMemo(() => new THREE.Raycaster(), []);
  const flightRef = useRef<{ from: CameraPose; to: CameraPose; start: number } | null>(null);
  const dragStart = useRef(1);
  const framed = useRef(false);
  /** A widened preview field of view and the stop's own, so poses report the stop's. */
  const fitted = useRef<{ shown: number; real: number } | null>(null);
  const report = useCallback(() => {
    const pose = poseOf(camera, orbit.target);
    const f = fitted.current;
    if (f && Math.abs(pose.fov - f.shown) < 1e-4) pose.fov = f.real;
    live.current.onPose(pose);
  }, [camera, orbit]);

  // The scene: backdrop, lights, the models' holders, the selection box.
  useEffect(() => {
    scene.background = new THREE.Color(BACKDROP);
    scene.add(world, stopsGroup, box, transform.getHelper());
    box.visible = false;
    return () => {
      scene.remove(world, stopsGroup, box, transform.getHelper());
    };
  }, [scene, world, stopsGroup, box, transform]);

  useEffect(() => {
    if (!holders) return;
    for (const holder of holders.values()) world.add(holder);
    live.current.onReady();
    return () => {
      for (const holder of holders.values()) world.remove(holder);
    };
  }, [holders, world]);

  // Each holder where the draft puts it.
  useEffect(() => {
    if (!holders) return;
    for (const model of props.models) {
      const holder = holders.get(model.asset);
      if (holder) applyPlacement(holder, placementOf(props.layout, model));
    }
  }, [holders, props.models, props.layout]);

  // The scene's radius, for the lights, the grid and the stop glyphs. It
  // follows big changes only, so a drag does not rebuild the lights.
  const [span, setSpan] = useState(1);
  useEffect(() => {
    if (!holders) return;
    world.updateMatrixWorld(true);
    const bounds = new THREE.Box3().setFromObject(world);
    const desk = props.layout.desk;
    if (desk) {
      bounds.union(
        new THREE.Box3(
          new THREE.Vector3(-desk[0] / 2, 0, -desk[1] / 2),
          new THREE.Vector3(desk[0] / 2, 0, desk[1] / 2)
        )
      );
    }
    if (bounds.isEmpty()) return;
    const far = Math.max(
      ...bounds.min.toArray().map(Math.abs),
      ...bounds.max.toArray().map(Math.abs)
    );
    const radius = Math.max(0.5, far);
    setSpan(prev => (Math.abs(prev - radius) / prev > 0.2 ? radius : prev));
  }, [holders, world, props.models, props.layout.models, props.layout.desk]);

  // Lights, desk or shadow catcher, and grid, sized to the scene.
  const extent = span * 1.25;
  useEffect(() => {
    const rig = new THREE.Group();
    rig.add(new THREE.HemisphereLight('#f4efe6', '#2a2926', 0.9));
    const key = new THREE.DirectionalLight('#ffd9b5', 2.4);
    key.position.set(-extent * 0.9, extent * 1.6, extent * 1.1);
    key.castShadow = true;
    key.shadow.mapSize.set(2048, 2048);
    key.shadow.radius = 3;
    const cam = key.shadow.camera;
    cam.left = -extent;
    cam.right = extent;
    cam.top = extent;
    cam.bottom = -extent;
    cam.near = 0.05;
    cam.far = extent * 6;
    cam.updateProjectionMatrix();
    key.shadow.bias = -0.0004;
    key.shadow.normalBias = 0.004;
    const fill = new THREE.DirectionalLight('#ffe9d6', 0.7);
    fill.position.set(extent * 1.4, extent * 0.5, extent * 0.8);
    const rim = new THREE.DirectionalLight('#b8ccff', 1.2);
    rim.position.set(extent * 0.2, extent * 1.2, -extent * 1.6);
    rig.add(key, fill, rim);

    const desk = props.layout.desk;
    if (desk) {
      const slab = new THREE.Mesh(
        new THREE.BoxGeometry(desk[0], DESK_THICKNESS, desk[1]),
        new THREE.MeshStandardMaterial({ color: DESK, roughness: 0.75 })
      );
      slab.position.y = -DESK_THICKNESS / 2;
      slab.receiveShadow = true;
      slab.name = 'desk';
      rig.add(slab);
    } else {
      const catcher = new THREE.Mesh(
        new THREE.PlaneGeometry(extent * 6, extent * 6),
        new THREE.ShadowMaterial({ opacity: 0.3, depthWrite: false })
      );
      catcher.rotation.x = -Math.PI / 2;
      catcher.receiveShadow = true;
      rig.add(catcher);
    }
    if (props.grid) {
      // 10 cm cells on a desk, half metres for a bigger scene.
      const cell = extent > 3 ? 0.5 : 0.1;
      const size = Math.ceil((extent * 2.4) / cell) * cell;
      const grid = new THREE.GridHelper(size, Math.round(size / cell), LINE, LINE);
      grid.position.y = 0.0015;
      const material = grid.material as THREE.Material;
      material.transparent = true;
      material.opacity = 0.55;
      material.depthWrite = false;
      rig.add(grid);
    }
    scene.add(rig);
    return () => {
      scene.remove(rig);
      disposeTree(rig);
    };
  }, [scene, extent, props.layout.desk, props.grid]);

  // The stops as little cameras; the selected one in clay, the first in sage.
  useEffect(() => {
    const sel = props.selection;
    props.layout.stops.forEach((stop, i) => {
      const color = sel?.kind === 'stop' && sel.id === stop.id ? CLAY : i === 0 ? SAGE : MUTE;
      stopsGroup.add(stopGlyph(stop, color, i === 0, span));
    });
    // Looking through a stop hides the glyphs in the way.
    stopsGroup.visible = !props.preview;
    return () => {
      for (const child of [...stopsGroup.children]) {
        stopsGroup.remove(child);
        disposeTree(child);
      }
    };
  }, [props.layout.stops, props.selection, props.preview, stopsGroup, span]);

  // The gizmo follows the selection: a model's holder or a stop's camera.
  useEffect(() => {
    const sel = props.selection;
    let target: THREE.Object3D | undefined;
    if (sel?.kind === 'model') target = holders?.get(sel.id);
    if (sel?.kind === 'stop') {
      target = stopsGroup.children.find(g => g.userData.stop === sel.id);
    }
    if (target && !props.preview) transform.attach(target);
    else transform.detach();
    box.visible = Boolean(sel?.kind === 'model' && target);
  }, [
    props.selection,
    props.preview,
    holders,
    stopsGroup,
    transform,
    box,
    props.layout.stops,
    span,
  ]);

  useEffect(() => {
    const stop = props.selection?.kind === 'stop';
    transform.setMode(stop ? 'translate' : props.tool);
    transform.setSpace(props.tool === 'rotate' ? 'local' : 'world');
    transform.setTranslationSnap(props.snap ? SNAP.move : null);
    transform.setRotationSnap(props.snap ? deg(SNAP.rotate) : null);
    transform.setScaleSnap(props.snap ? SNAP.scale : null);
    transform.setSize(0.8);
  }, [transform, props.tool, props.snap, props.selection]);

  useEffect(() => {
    const onDragging = (e: { value: unknown }) => {
      orbit.enabled = !e.value;
      const object = transform.object;
      if (e.value) {
        dragStart.current = object?.scale.x ?? 1;
        live.current.onBeginDrag();
      } else if (object?.userData.stop) {
        // A stop rebuilds its glyph when it changes, so it is saved once, at the end.
        live.current.onDragStop(object.userData.stop as string, object.position.toArray() as Vec3);
      }
    };
    const onChange = () => {
      const object = transform.object;
      const p = live.current;
      if (!object || object.userData.stop) return;
      const asset = object.userData.asset as string | undefined;
      if (!asset) return;
      if (transform.getMode() === 'scale') {
        // Scale stays uniform: the axis pulled furthest sets all three.
        const s = object.scale;
        const was = dragStart.current;
        const next = [s.x, s.y, s.z].reduce((a, b) =>
          Math.abs(b - was) > Math.abs(a - was) ? b : a
        );
        object.scale.setScalar(Math.max(0.01, next));
      }
      p.onDragModel(asset, placementFrom(object));
    };
    transform.addEventListener('dragging-changed', onDragging);
    transform.addEventListener('objectChange', onChange);
    return () => {
      transform.removeEventListener('dragging-changed', onDragging);
      transform.removeEventListener('objectChange', onChange);
      transform.detach();
      transform.dispose();
    };
  }, [transform, orbit]);

  useEffect(() => {
    orbit.enableDamping = true;
    orbit.dampingFactor = 0.1;
    orbit.screenSpacePanning = true;
    const start = () => {
      flightRef.current = null;
      live.current.onUserMove();
    };
    const end = () => report();
    orbit.addEventListener('start', start);
    orbit.addEventListener('end', end);
    return () => {
      orbit.removeEventListener('start', start);
      orbit.removeEventListener('end', end);
      orbit.dispose();
    };
  }, [orbit, camera, report]);

  // Frame every model: on first load and when asked.
  useEffect(() => {
    if (!holders || holders.size === 0) return;
    if (framed.current && props.frame === 0) return;
    framed.current = true;
    world.updateMatrixWorld(true);
    const bounds = new THREE.Box3().setFromObject(world);
    if (props.layout.desk) {
      const [w, d] = props.layout.desk;
      bounds.union(
        new THREE.Box3(new THREE.Vector3(-w / 2, 0, -d / 2), new THREE.Vector3(w / 2, 0, d / 2))
      );
    }
    if (bounds.isEmpty()) return;
    camera.fov = 35;
    frameCamera(camera, {
      box: bounds,
      extent: bounds.getSize(new THREE.Vector3()).length() || 1,
      target: bounds.getCenter(new THREE.Vector3()),
    });
    camera.near = 0.01;
    camera.far = 200;
    camera.updateProjectionMatrix();
    orbit.target.copy(bounds.getCenter(new THREE.Vector3()));
    flightRef.current = null;
    live.current.onPose(poseOf(camera, orbit.target));
    // Only loading and the frame counter frame again, not every edit.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [holders, props.frame]);

  // Looking through a stop: fly there.
  useEffect(() => {
    if (!props.preview) return;
    const stop = live.current.layout.stops.find(s => s.id === props.preview?.id);
    if (!stop) return;
    const shown = fitFov(stop.fov, camera.aspect);
    fitted.current = { shown, real: stop.fov };
    flightRef.current = {
      from: poseOf(camera, orbit.target),
      to: { position: stop.position, target: stop.target, fov: shown },
      start: performance.now(),
    };
    // Only a new click starts a flight.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.preview?.at]);

  // While looking through a stop, edits to it show at once.
  const previewStop = props.preview
    ? props.layout.stops.find(s => s.id === props.preview?.id)
    : undefined;
  useEffect(() => {
    if (!previewStop || flightRef.current) return;
    const shown = fitFov(previewStop.fov, camera.aspect);
    fitted.current = { shown, real: previewStop.fov };
    setPose(camera, orbit, {
      position: previewStop.position,
      target: previewStop.target,
      fov: shown,
    });
  }, [previewStop, camera, orbit, size.width, size.height]);

  // A click (not a drag, not on the gizmo) selects what is under the pointer.
  useEffect(() => {
    const el = gl.domElement;
    let down: { x: number; y: number } | null = null;
    const onDown = (e: PointerEvent) => {
      down = { x: e.clientX, y: e.clientY };
    };
    const onUp = (e: PointerEvent) => {
      if (!down || Math.hypot(e.clientX - down.x, e.clientY - down.y) > 4) return;
      down = null;
      if (transform.dragging || transform.axis) return;
      const rect = el.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1
      );
      raycaster.setFromCamera(ndc, camera);
      const targets = [world, ...(stopsGroup.visible ? [stopsGroup] : [])];
      const hits = raycaster.intersectObjects(targets, true);
      for (const hit of hits) {
        let node: THREE.Object3D | null = hit.object;
        while (node && node.userData.asset === undefined && node.userData.stop === undefined) {
          node = node.parent;
        }
        if (node?.userData.stop !== undefined && hit.object.name === 'stop-body') {
          live.current.onSelect({ kind: 'stop', id: node.userData.stop as string });
          return;
        }
        if (node?.userData.asset !== undefined) {
          live.current.onSelect({ kind: 'model', id: node.userData.asset as string });
          return;
        }
      }
      live.current.onSelect(null);
    };
    el.addEventListener('pointerdown', onDown);
    el.addEventListener('pointerup', onUp);
    return () => {
      el.removeEventListener('pointerdown', onDown);
      el.removeEventListener('pointerup', onUp);
    };
  }, [gl, camera, raycaster, world, stopsGroup, transform]);

  useFrame(() => {
    const f = flightRef.current;
    if (f) {
      const elapsed = performance.now() - f.start;
      setPose(camera, orbit, flight(f.from, { ...f.to, name: '', poster: null }, elapsed));
      if (elapsed >= FLIGHT_MS) {
        flightRef.current = null;
        report();
      }
    } else {
      orbit.update();
    }
    const sel = live.current.selection;
    if (sel?.kind === 'model' && box.visible) {
      const holder = holders?.get(sel.id);
      if (holder) box.box.setFromObject(holder);
    }
  });

  return null;
}

export default function LayoutCanvas({ className, ...props }: LayoutCanvasProps) {
  return (
    <Canvas
      className={className}
      shadows
      camera={{ fov: 35, position: [1.4, 1, 1.6], near: 0.01, far: 200 }}
      dpr={[1, 2]}
      gl={{ antialias: true }}
      aria-label="Scene layout"
    >
      <Editor {...props} />
    </Canvas>
  );
}
