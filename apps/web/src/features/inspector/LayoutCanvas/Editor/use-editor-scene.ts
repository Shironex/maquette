import { useEffect, useState } from 'react';
import * as THREE from 'three';
import { DESK_THICKNESS, placementOf } from '@/state/layout';
import { BACKDROP, CLAY } from '@/features/viewer/studio';
import { disposeObject } from '@/features/viewer/dispose';
import type { EditorProps, EditorRig } from './Editor.types';
import { applyPlacement, applyPrep } from './placement';
import { stopGlyph } from './stop-glyph';

const LINE = '#4d4a44';
const MUTE = '#9a9488';
const SAGE = '#8fa58c';
const DESK = '#6e5a49';

/**
 * The editor's scene: backdrop, the models where the draft puts them, the
 * lights, desk and grid sized to the scene, and the stops as little
 * cameras. Returns the scene's radius.
 */
export function useEditorScene(editor: EditorRig, props: EditorProps): number {
  const { scene, holders, live, transform, world, stopsGroup, box } = editor;

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
    live.current.handlers.onReady();
    return () => {
      for (const holder of holders.values()) world.remove(holder);
    };
  }, [holders, world, live]);

  // Each holder where the draft puts it.
  useEffect(() => {
    if (!holders) return;
    for (const model of props.models) {
      const holder = holders.get(model.asset);
      if (!holder) continue;
      applyPrep(holder, model.prep);
      applyPlacement(holder, placementOf(props.layout, model));
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
      disposeObject(rig);
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
        disposeObject(child);
      }
    };
  }, [props.layout.stops, props.selection, props.preview, stopsGroup, span]);

  return span;
}
