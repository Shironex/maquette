/**
 * The Blender stop's wireframe sweep: a wire pass over the model's own
 * geometry (no second asset), shown left of a front that crosses the model,
 * with a bright edge at the front itself.
 */
import * as THREE from 'three';

export interface Sweep {
  /** 0 hides the wire, 1 shows it everywhere. */
  setProgress(progress: number): void;
  setVisible(visible: boolean): void;
}

const WIRE = new THREE.Color('#e9e5dc');
const EDGE = new THREE.Color('#e9b49c');

export function addSweep(model: THREE.Object3D): Sweep {
  const uniforms = {
    uFront: { value: -Infinity },
    uWire: { value: WIRE },
    uEdge: { value: EDGE },
  };
  const material = new THREE.MeshBasicMaterial({
    wireframe: true,
    transparent: true,
    depthWrite: false,
    toneMapped: false,
  });
  material.onBeforeCompile = shader => {
    Object.assign(shader.uniforms, uniforms);
    shader.vertexShader = shader.vertexShader
      .replace('#include <common>', '#include <common>\nvarying float vSweepX;')
      .replace(
        '#include <project_vertex>',
        '#include <project_vertex>\nvSweepX = (modelMatrix * vec4(transformed, 1.0)).x;'
      );
    shader.fragmentShader = shader.fragmentShader
      .replace(
        '#include <common>',
        '#include <common>\nvarying float vSweepX;\nuniform float uFront;\nuniform vec3 uWire, uEdge;'
      )
      .replace(
        '#include <dithering_fragment>',
        `#include <dithering_fragment>
float behind = uFront - vSweepX;
if (behind < 0.0) discard;
float edge = 1.0 - smoothstep(0.0, 0.012, behind);
gl_FragColor = vec4(mix(uWire, uEdge, edge), mix(0.32, 0.95, edge));`
      );
  };

  const meshes: THREE.Mesh[] = [];
  model.traverse(child => {
    if ((child as THREE.Mesh).isMesh) meshes.push(child as THREE.Mesh);
  });
  const wires = meshes.map(mesh => {
    const wire = new THREE.Mesh(mesh.geometry, material);
    wire.renderOrder = 2;
    wire.visible = false;
    mesh.add(wire);
    return wire;
  });

  // Measured in world space on first use, once the model stands in the scene.
  let span: { from: number; width: number } | null = null;
  return {
    setProgress(progress) {
      if (!span) {
        model.updateWorldMatrix(true, true);
        const bounds = new THREE.Box3().setFromObject(model);
        const pad = (bounds.max.x - bounds.min.x) * 0.06;
        span = { from: bounds.min.x - pad, width: bounds.max.x - bounds.min.x + pad * 2 };
      }
      uniforms.uFront.value = span.from + span.width * progress;
    },
    setVisible(visible) {
      for (const wire of wires) wire.visible = visible;
    },
  };
}
