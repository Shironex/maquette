/**
 * True when the system asks for less motion. The 3D views read it to jump
 * to a camera stop instead of flying there and to leave the turntable off;
 * CSS animations follow the same setting in the kit's base layer.
 */
export function reducedMotion(): boolean {
  return typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** When a camera flight starts: now, or a whole flight ago, so it lands at once. */
export function flightStart(durationMs: number): number {
  return performance.now() - (reducedMotion() ? durationMs : 0);
}
