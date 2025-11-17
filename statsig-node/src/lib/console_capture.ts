import {
  getStackTrace,
  safeStringify,
  wrapFunctionWithRestore,
} from './console_captrue_utils';

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

function captureLog(
  level: CaptureLevel,
  args: unknown[],
  sdkKey: string
): void {
  const message = args.map((a) => safeStringify(a, 100, 10, 1000));

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

export function startStatsigConsoleCapture(sdkKey: string): void {
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

          try {
            captureLog(level, args, sdkKey);
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
