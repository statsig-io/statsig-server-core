import {
  getStackTrace,
  safeStringify,
  wrapFunctionWithRestore,
} from './console_captrue_utils';

import { ConsoleCaptureOptions } from './statsig-generated';
import { statsigCaptureLogLine } from '.';

const CAPTURE_LEVELS = [
  'log',
  'warn',
  'error',
  'debug',
  'info',
  'trace',
] as const;
type CaptureLevel = (typeof CAPTURE_LEVELS)[number];

const originalConsoleFns: Partial<
  Record<CaptureLevel, (...args: unknown[]) => void>
> = {};
const originalConsoleErrorFn =
  typeof console !== 'undefined' ? console.error : undefined;

const MAX_KEYS = 100;
const MAX_DEPTH = 10;
const MAX_LENGTH = 4096;

function captureLog(
  level: CaptureLevel,
  args: unknown[],
  sdkKey: string,
  maxKeys: number,
  maxDepth: number,
  maxLength: number
): void {
  const message = args.map((a) =>
    safeStringify(a, maxKeys, maxDepth, maxLength)
  );

  if (
    level.toLowerCase() === 'error' &&
    typeof args[0] === 'string' &&
    args[0].startsWith('Trace')
  ) {
    level = 'trace'; // node use error for console.trace
  }

  const stackTrace = getStackTrace();
  statsigCaptureLogLine(level, message, sdkKey, stackTrace);
}

export function startStatsigConsoleCapture(
  sdkKey: string,
  options?: ConsoleCaptureOptions
): void {
  stopStatsigConsoleCapture();

  for (const level of CAPTURE_LEVELS) {
    const originalFn = console[level];
    if (!originalFn || typeof originalFn !== 'function') {
      continue;
    }

    let isCapturing = false;

    const restoreFn = wrapFunctionWithRestore(
      console as unknown as Record<string, unknown>,
      level,
      (originalFn) => {
        return (...args: unknown[]) => {
          originalFn(...args);

          if (isCapturing) return;
          isCapturing = true;

          const maxKeys = Math.min(options?.maxKeys ?? MAX_KEYS, MAX_KEYS);
          const maxDepth = Math.min(options?.maxDepth ?? MAX_DEPTH, MAX_DEPTH);
          const maxLength = Math.min(
            options?.maxLength ?? MAX_LENGTH,
            MAX_LENGTH
          );

          try {
            captureLog(level, args, sdkKey, maxKeys, maxDepth, maxLength);
          } catch (err) {
            if (
              originalConsoleErrorFn &&
              typeof originalConsoleErrorFn === 'function'
            ) {
              originalConsoleErrorFn('Statsig log capture failed:', err);
            }
          } finally {
            isCapturing = false;
          }
        };
      }
    );

    originalConsoleFns[level] = restoreFn;
  }
}

export function stopStatsigConsoleCapture(): void {
  for (const level of CAPTURE_LEVELS) {
    const restoreFn = originalConsoleFns[level];
    if (restoreFn && typeof restoreFn === 'function') {
      restoreFn();
      delete originalConsoleFns[level];
    }
  }

  for (const key of Object.keys(originalConsoleFns)) {
    delete originalConsoleFns[key as keyof typeof originalConsoleFns];
  }
}
