/**
 * A stand in for react-three-fiber under jsdom, which has no WebGL. `useThree`
 * reads a fake root state (a real camera and scene, a renderer that records
 * calls), `useFrame` keeps its callbacks so a test steps frames by hand, and
 * `Canvas` renders a plain element and keeps its props.
 *
 *   vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));
 */
import { createElement, useEffect, useLayoutEffect, useRef, type ReactNode } from 'react';
import * as THREE from 'three';
import { vi } from 'vitest';

export interface FakeState {
  gl: THREE.WebGLRenderer;
  camera: THREE.PerspectiveCamera;
  scene: THREE.Scene;
  size: { width: number; height: number };
}

type FrameCallback = (state: FakeState, delta: number) => void;

export interface FakeCanvasProps {
  children?: ReactNode;
  className?: string;
  'aria-label'?: string;
  [prop: string]: unknown;
}

const WIDTH = 800;
const HEIGHT = 450;

function fakeRenderer() {
  const domElement = document.createElement('canvas');
  domElement.width = WIDTH;
  domElement.height = HEIGHT;
  domElement.getBoundingClientRect = () => new DOMRect(0, 0, WIDTH, HEIGHT);
  // Orbit and transform controls capture the pointer, which jsdom does not do.
  domElement.setPointerCapture = () => {};
  domElement.releasePointerCapture = () => {};
  const context = { RGBA: 0, UNSIGNED_BYTE: 0, readPixels: vi.fn() };
  return {
    domElement,
    render: vi.fn(),
    setViewport: vi.fn(),
    setScissor: vi.fn(),
    setScissorTest: vi.fn(),
    getContext: () => context,
  };
}

function freshState(): FakeState {
  return {
    gl: fakeRenderer() as unknown as THREE.WebGLRenderer,
    camera: new THREE.PerspectiveCamera(30, WIDTH / HEIGHT),
    scene: new THREE.Scene(),
    size: { width: WIDTH, height: HEIGHT },
  };
}

export const fiber = {
  state: freshState(),
  frames: new Set<FrameCallback>(),
  canvas: null as FakeCanvasProps | null,
};

/** A new fake root: a fresh camera, scene and renderer, no frame callbacks. */
export function resetFiber(): FakeState {
  fiber.state = freshState();
  fiber.frames.clear();
  fiber.canvas = null;
  return fiber.state;
}

/** Runs every mounted `useFrame` callback once. */
export function step(delta = 1 / 60): void {
  for (const frame of [...fiber.frames]) frame(fiber.state, delta);
}

export function useThree<T = FakeState>(pick?: (state: FakeState) => T): T {
  return pick ? pick(fiber.state) : (fiber.state as T);
}

export function useFrame(callback: FrameCallback, _priority?: number): void {
  const latest = useRef(callback);
  useLayoutEffect(() => {
    latest.current = callback;
  });
  useEffect(() => {
    const run: FrameCallback = (state, delta) => latest.current(state, delta);
    fiber.frames.add(run);
    return () => {
      fiber.frames.delete(run);
    };
  }, []);
}

export function Canvas(props: FakeCanvasProps) {
  fiber.canvas = props;
  return createElement('div', {
    'data-testid': 'canvas',
    className: props.className,
    'aria-label': props['aria-label'],
  });
}
