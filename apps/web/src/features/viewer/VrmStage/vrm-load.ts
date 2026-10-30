/**
 * Loading a `.vrm` with three-vrm on top of the viewer's glTF loader, and
 * what the loaded file says about itself.
 */
import type * as THREE from 'three';
import type { GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import {
  VRMHumanBoneList,
  VRMLoaderPlugin,
  VRMLookAtBoneApplier,
  VRMUtils,
  type VRM,
} from '@pixiv/three-vrm';
import { gltfLoader } from '../studio';
import { REQUIRED_BONES } from '../vrm-pose';
import type { VrmSummary } from './VrmStage.types';

/** The look expressions an expression look at drives. */
const LOOKS = ['lookLeft', 'lookRight', 'lookUp', 'lookDown'];

/** The viewer's glTF loader (meshopt, KTX2) that also reads VRM. */
export function vrmLoader(renderer: THREE.WebGLRenderer) {
  return gltfLoader(renderer).register(parser => new VRMLoaderPlugin(parser));
}

/**
 * The VRM a loaded glTF holds, turned to face +Z when it is a 0.x file
 * (VRM 1.0 already does); null for a plain glTF.
 */
export function vrmOf(gltf: GLTF): VRM | null {
  const vrm = (gltf.userData as { vrm?: VRM }).vrm ?? null;
  if (vrm?.meta.metaVersion === '0') VRMUtils.rotateVRM0(vrm);
  return vrm;
}

interface DeclaredLookAt {
  extensions?: {
    VRMC_vrm?: { lookAt?: { type?: string } };
    VRM?: { firstPerson?: { lookAtTypeName?: string } };
  };
}

/**
 * The look at type the file declares. three-vrm makes no look at for an
 * expression look at in a file with no expressions, so the JSON says it.
 */
function declaredLookAt(gltf: GLTF): VrmSummary['lookAt'] {
  const json = (gltf.parser.json ?? {}) as DeclaredLookAt;
  const type =
    json.extensions?.VRMC_vrm?.lookAt?.type ?? json.extensions?.VRM?.firstPerson?.lookAtTypeName;
  if (!type) return 'none';
  return type.toLowerCase() === 'bone' ? 'bone' : 'expression';
}

/** What `vrm`, loaded from `gltf`, holds. */
export function summarize(gltf: GLTF, vrm: VRM): VrmSummary {
  const humanoid = vrm.humanoid;
  const bones = VRMHumanBoneList.filter(name => humanoid.getRawBoneNode(name)).length;
  const missing = REQUIRED_BONES.filter(name => !humanoid.getRawBoneNode(name));
  const expressions = vrm.expressionManager?.expressions ?? [];
  const applier = vrm.lookAt?.applier;
  const lookAt = !applier
    ? declaredLookAt(gltf)
    : applier instanceof VRMLookAtBoneApplier
      ? 'bone'
      : 'expression';
  const lookAtMoves =
    applier instanceof VRMLookAtBoneApplier ||
    (applier !== undefined &&
      LOOKS.some(name => (vrm.expressionManager?.getExpression(name)?.binds.length ?? 0) > 0));
  return {
    spec: vrm.meta.metaVersion === '0' ? '0.x' : '1.0',
    bones,
    missing,
    springs: vrm.springBoneManager?.joints.size ?? 0,
    expressions: expressions.length,
    lookAt,
    lookAtMoves,
  };
}
