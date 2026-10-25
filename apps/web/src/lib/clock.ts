/**
 * The one place the app reads the wall clock. Everything else asks here, so a
 * test can fake time with `vi.useFakeTimers()` and `vi.setSystemTime()`.
 */
export const clock = {
  /** Milliseconds since the epoch. */
  nowMs(): number {
    return Date.now();
  },
  /** The current time as a `Date`. */
  now(): Date {
    return new Date();
  },
};
