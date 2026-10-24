/**
 * The one import the UI uses for IPC. The generated bindings stay untouched;
 * this module adds the error shape and re-exports the types.
 */
export { commands } from './generated/bindings';
export type * from './generated/bindings';

import type { ErrorPayload } from './generated/bindings';

export function isErrorPayload(value: unknown): value is ErrorPayload {
  return (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    'message' in value &&
    typeof (value as ErrorPayload).message === 'string'
  );
}

/** A readable sentence for any rejection: a command error, an Error or a string. */
export function errorMessage(error: unknown): string {
  if (isErrorPayload(error)) return error.message;
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return 'Something went wrong.';
}
