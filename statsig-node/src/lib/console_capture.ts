import { safeStringify } from "./console_captrue_utils";
import { statsigCaptureLogLine } from ".";

const methodsToWrap = ["log", "warn", "error", "debug"] as const;
type ConsoleMethod = (typeof methodsToWrap)[number];

const originals: Partial<Record<ConsoleMethod, (...args: unknown[]) => void>> = {};

function capture(method: ConsoleMethod, args: unknown[], sdkKey: string): void {
  try {
    const message = args
      .map((a) => safeStringify(a, 100, 10, 1000))

      statsigCaptureLogLine(method, message, sdkKey);
  } catch (err) {
    originals[method]?.("Statsig log capture failed:", err);
  }
}

export function startStatsigConsoleCapture(sdkKey: string): void {
  for (const method of methodsToWrap) {
    if (!originals[method] || typeof originals[method] !== "function") {
      originals[method] = console[method];
    }

    console[method] = (...args: unknown[]) => {
      capture(method, args, sdkKey);
      originals[method]?.(...args);
    };
  }
}
