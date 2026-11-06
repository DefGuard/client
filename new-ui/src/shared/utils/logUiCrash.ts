import { isNotFound, isRedirect } from '@tanstack/react-router';
import { error as logError } from '@tauri-apps/plugin-log';

export const UiCrashSource = {
  Caught: 'caught',
  Uncaught: 'uncaught',
  Window: 'window',
} as const;

export type UiCrashSourceValue = (typeof UiCrashSource)[keyof typeof UiCrashSource];

const summaryOf = (thrown: unknown): string => {
  if (thrown instanceof Error) return `${thrown.name}: ${thrown.message}`;
  if (typeof thrown === 'string') return thrown;
  try {
    return JSON.stringify(thrown) ?? String(thrown);
  } catch {
    return String(thrown);
  }
};

const stackOf = (thrown: unknown): string | undefined => {
  if (!(thrown instanceof Error) || !thrown.stack) return undefined;
  // V8 starts the stack with the summary line, WebKit does not
  const summary = summaryOf(thrown);
  const stack = thrown.stack.startsWith(summary)
    ? thrown.stack.slice(summary.length)
    : thrown.stack;
  return stack.trim() || undefined;
};

export const logUiCrash = (
  source: UiCrashSourceValue,
  thrown: unknown,
  componentStack?: string,
): void => {
  // router control flow travels through React error boundaries
  if (isRedirect(thrown) || isNotFound(thrown)) return;

  const stack = stackOf(thrown);
  const components = componentStack?.trim();
  // only the first line matches the Logs page parser, the rest stays in the log file
  const message = [
    `UI crash (${source}): ${summaryOf(thrown)}`,
    stack,
    components ? `Component stack:\n${components}` : undefined,
  ]
    .filter((part) => part !== undefined)
    .join('\n');

  logError(message).catch((err: unknown) => {
    console.error('Failed to write UI crash to log', err);
  });
};
