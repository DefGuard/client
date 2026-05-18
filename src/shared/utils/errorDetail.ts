/**
 * Extracts the most useful debug string from an unknown caught value.
 *
 * - For Error objects: returns the stack trace (includes message + call chain).
 *   Falls back to message if stack is unavailable.
 * - For anything else (string, number, …): coerces to string.
 */
export const errorDetail = (e: unknown): string =>
  e instanceof Error ? (e.stack ?? e.message) : String(e);
