export type Counter = {
  sdkType: string;
  sdkVersion: string;
  count: number;
} & (
  | {
      kind: 'req';
      path: string;
      method: string;
    }
  | {
      kind: 'event';
    }
);

let reqCounts: Record<string, Counter> = {};
let eventCounts: Record<string, Counter> = {};

export function incReqCount(
  sdkType: string,
  sdkVersion: string,
  path: string,
  method: string,
) {
  const key = [sdkType, sdkVersion, path, method].join('|');
  const curr = reqCounts[key]?.count ?? 0;
  reqCounts[key] = {
    kind: 'req',
    sdkType,
    sdkVersion,
    path,
    method,
    count: curr + 1,
  };
}

export function incEventCount(
  sdkType: string,
  sdkVersion: string,
  count: number,
) {
  const key = [sdkType, sdkVersion].join('|');
  const curr = eventCounts[key]?.count ?? 0;
  eventCounts[key] = {
    kind: 'event',
    sdkType,
    sdkVersion,
    count: curr + count,
  };
}

export function takeCounters(): Counter[] {
  const localReqCounts = reqCounts;
  reqCounts = {};

  const localEventCounts = eventCounts;
  eventCounts = {};

  return [...Object.values(localReqCounts), ...Object.values(localEventCounts)];
}
