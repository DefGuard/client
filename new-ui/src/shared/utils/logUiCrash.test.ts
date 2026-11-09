import { notFound, redirect } from '@tanstack/react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { logUiCrash, UiCrashSource } from './logUiCrash';

const mocks = vi.hoisted(() => ({ error: vi.fn() }));

vi.mock('@tauri-apps/plugin-log', () => ({ error: mocks.error }));

const loggedMessage = (): string => mocks.error.mock.calls[0][0];

describe('logUiCrash', () => {
  beforeEach(() => {
    mocks.error.mockReset();
    mocks.error.mockResolvedValue(undefined);
  });

  it('logs the summary first, then the stack and component stack', () => {
    const err = new TypeError('boom');
    err.stack = 'TypeError: boom\n    at Overview (overview.tsx:1:1)';

    logUiCrash(UiCrashSource.Caught, err, '\n    at Overview\n    at App');

    expect(loggedMessage()).toBe(
      [
        'UI crash (caught): TypeError: boom',
        'at Overview (overview.tsx:1:1)',
        'Component stack:',
        'at Overview\n    at App',
      ].join('\n'),
    );
  });

  it('keeps a stack that does not repeat the summary', () => {
    const err = new Error('boom');
    err.stack = 'Overview@overview.tsx:1:1';

    logUiCrash(UiCrashSource.Window, err);

    expect(loggedMessage()).toBe(
      'UI crash (window): Error: boom\nOverview@overview.tsx:1:1',
    );
  });

  it('describes non-Error values', () => {
    logUiCrash(UiCrashSource.Caught, 'plain string');
    logUiCrash(UiCrashSource.Caught, { code: 42 });
    logUiCrash(UiCrashSource.Caught, undefined);

    expect(mocks.error.mock.calls.map(([message]) => message)).toEqual([
      'UI crash (caught): plain string',
      'UI crash (caught): {"code":42}',
      'UI crash (caught): undefined',
    ]);
  });

  it('ignores router redirects and not-found errors', () => {
    logUiCrash(UiCrashSource.Caught, redirect({ to: '/' }));
    logUiCrash(UiCrashSource.Caught, notFound());

    expect(mocks.error).not.toHaveBeenCalled();
  });

  it('does not throw when writing the log fails', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    mocks.error.mockRejectedValue(new Error('ipc down'));

    expect(() => logUiCrash(UiCrashSource.Uncaught, new Error('boom'))).not.toThrow();
    await vi.waitFor(() => expect(consoleError).toHaveBeenCalled());

    consoleError.mockRestore();
  });
});
