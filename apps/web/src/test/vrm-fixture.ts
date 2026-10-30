/**
 * The mock `.vrm` (the box humanoid test fixture written as VRM 1.0), parsed
 * with three-vrm in node. jsdom has no WebGL and loads no images, so every
 * texture comes back blank; the rest loads as an app would load it.
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import * as THREE from 'three';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRMLoaderPlugin } from '@pixiv/three-vrm';

export const FIXTURE_VRM = resolve(import.meta.dirname, '../assets/mock/avatar/box_humanoid.vrm');

/** A loader plugin that answers every texture with a blank one. */
const blankTextures = () =>
  ({
    name: 'blank_textures',
    loadTexture: () => Promise.resolve(new THREE.Texture()),
  }) as never;

/** `path` as an ArrayBuffer of this realm, which GLTFLoader checks for. */
function bytesOf(path: string): ArrayBuffer {
  const file = readFileSync(path);
  const out = new ArrayBuffer(file.byteLength);
  new Uint8Array(out).set(file);
  return out;
}

/** Parses the `.vrm` at `path` (the fixture without one), with or without the VRM plugin. */
export function parseVrm(path = FIXTURE_VRM, vrm = true): Promise<GLTF> {
  const loader = new GLTFLoader();
  if (vrm) loader.register(parser => new VRMLoaderPlugin(parser));
  loader.register(blankTextures);
  return loader.parseAsync(bytesOf(path), '');
}
