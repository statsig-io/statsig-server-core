import type { Request } from 'express';

export const BAD_SDK_TYPE = 'BAD_SDK_TYPE';
export const BAD_SDK_VERSION = 'BAD_SDK_VERSION';

export function getSdkInfo(req: Request) {
  const sdkType = String(req.headers?.['statsig-sdk-type'] ?? BAD_SDK_TYPE);
  const sdkVersion = String(
    req.headers?.['statsig-sdk-version'] ?? BAD_SDK_VERSION,
  );

  return {
    sdkType,
    sdkVersion,
  };
}

export async function logEventsToStatsig(events: any[], sdkKey: string) {
  await fetch('https://events.statsigapi.net/v1/log_event', {
    method: 'POST',
    body: JSON.stringify({
      events,
    }),
    headers: {
      'STATSIG-API-KEY': sdkKey,
    },
  });
}

export function log(message: string, ...args: unknown[]) {
  console.log(`[${new Date().toISOString()}][scrapi] ${message}`, ...args);
}
