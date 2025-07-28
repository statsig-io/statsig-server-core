let reqCounts = {};
let eventCounts = {};

export function incReqCount(
  sdkType: string,
  sdkVersion: string,
  path: string,
  method: string,
) {
  const key = [sdkType, sdkVersion, path, method].join('|');
  reqCounts[key] = (reqCounts[key] ?? 0) + 1;
}

export function incEventCount(
  sdkType: string,
  sdkVersion: string,
  count: number,
) {
  const key = [sdkType, sdkVersion].join('|');
  eventCounts[key] = (eventCounts[key] ?? 0) + count;
}

export function takeCounters() {
  const localReqCounts = reqCounts;
  reqCounts = {};

  const localEventCounts = eventCounts;
  eventCounts = {};

  return {
    reqCounts: localReqCounts,
    eventCounts: localEventCounts,
  };
}
