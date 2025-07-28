import express, { type Request, type Response } from 'express';
import { readFileSync } from 'node:fs';

import type { ScrapiState } from '../common';
import { incEventCount, incReqCount, takeCounters } from './counters';
import { getSdkInfo } from './utils';

const benchmarkSdkKey: string = process.env.BENCH_CLUSTER_SDK_KEY ?? '';
if (!benchmarkSdkKey || benchmarkSdkKey === '') {
  throw new Error('BENCH_CLUSTER_SDK_KEY is not set');
}

const app = express();
let state: ScrapiState | null = null;
let lastFlushedAt = new Date();

app.use((req, _res, next) => {
  const { sdkType, sdkVersion } = getSdkInfo(req);
  incReqCount(sdkType, sdkVersion, req.path, req.method);

  console.log(`${req.method} ${req.path} from ${sdkType}@${sdkVersion}`);
  next();
});

app.post('/v1/log_event', async (req, res) => {
  const { sdkType, sdkVersion } = getSdkInfo(req);
  const eventCountStr =
    req.headers?.['statsig-event-count'] ?? req.body?.events?.length;

  if (!eventCountStr) {
    throw new Error('statsig-event-count is required');
  }

  const logEventState = state?.logEvent;
  if (logEventState == null) {
    throw new Error('logEvent state not found');
  }

  const delayMs = logEventState?.response?.delayMs ?? 0;
  if (delayMs > 0) {
    await new Promise((r) => setTimeout(r, delayMs));
  }

  if (logEventState.response.status > 300) {
    res
      .status(logEventState.response.status)
      .json(JSON.parse(logEventState.response.payload));
    return;
  }

  const eventCount = parseInt(eventCountStr);
  if (isNaN(eventCount)) {
    throw new Error('statsig-event-count is not a number');
  }

  incEventCount(sdkType, sdkVersion, eventCount);

  res
    .status(logEventState.response.status)
    .json(JSON.parse(logEventState.response.payload));
});

app.all('/v2/download_config_specs/:sdk_key', (_req, res) =>
  handleDcsResponse('v2', res),
);

app.all(
  ['/v1/download_config_specs/:sdk_key', '/v1/download_config_specs'],
  (_req, res) => handleDcsResponse('v1', res),
);

app.all('/v1/get_id_lists', (_req, res) => {
  res.status(200).json({});
});

app.all('/v1/download_id_list_file/:id_list_name', async (_req, res) => {
  res.status(404).json({ error: 'ID list not found' });
});

app.listen(8000, () => {
  console.log('Server is running on port 8000');
  setInterval(update, 1000).unref();
});

async function handleDcsResponse(type: 'v1' | 'v2', res: Response) {
  const delayMs = state?.dcs?.response?.delayMs ?? 0;
  if (delayMs > 0) {
    await new Promise((r) => setTimeout(r, delayMs));
  }

  if (state?.dcs?.response?.status !== 200) {
    res.status(state?.dcs?.response?.status ?? 500).json({
      error: 'DCS is not enabled',
    });
    return;
  }

  const payload =
    type === 'v2'
      ? state?.dcs?.response?.v2Payload
      : state?.dcs?.response?.v1Payload;
  if (payload == null || payload.length === 0) {
    res.status(404).json({ error: 'State not initialized' });
    return;
  }

  res.status(200).json(JSON.parse(payload));
}

function readState(): ScrapiState {
  const stateContents = readFileSync('/shared-volume/state.json', 'utf8');
  return JSON.parse(stateContents).scrapi as ScrapiState;
}

function update() {
  state = readState();

  if (Date.now() - lastFlushedAt.getTime() > 10_000) {
    const counters = takeCounters();
    if (
      Object.keys(counters.reqCounts).length > 0 ||
      Object.keys(counters.eventCounts).length > 0
    ) {
      console.log(JSON.stringify(counters, null, 2));
    }

    lastFlushedAt = new Date();
  }

  // logEventsToStatsig(counters.reqCounts, benchmarkSdkKey);
}
