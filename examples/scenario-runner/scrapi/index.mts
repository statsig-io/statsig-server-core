import express, { type Response } from 'express';
import { createReadStream, readFileSync } from 'node:fs';

import type { ScrapiState, State } from '../common';
import { incEventCount, incReqCount, takeCounters } from './counters';
import {
  flushCounters,
  flushDockerStats,
  getSdkInfo,
  logStateChange,
} from './utils';

const app = express();
let scrapiState: ScrapiState | null = null;
let lastFlushedAt = new Date();
let lastStateUpdateAt = new Date(1);

app.all('/ready', (_req, res) => {
  const v1Filepath = scrapiState?.dcs?.response?.v1?.filepath;
  const v2Filepath = scrapiState?.dcs?.response?.v2?.filepath;

  if (v1Filepath == null || v2Filepath == null) {
    res.status(500).json({ error: 'State not initialized' });
    return;
  }

  res.status(200).json({
    v1DcsBytesize: scrapiState?.dcs?.response?.v1?.filesize,
    v2DcsBytesize: scrapiState?.dcs?.response?.v2?.filesize,
  });
});

app.use((req, _res, next) => {
  const { sdkType, sdkVersion } = getSdkInfo(req);
  incReqCount(sdkType, sdkVersion, req.path, req.method);

  console.log(`${req.method} ${req.path} from ${sdkType}@${sdkVersion}`);
  next();
});

app.use((req, res, next) => {
  const encoding = req.headers['content-encoding'] ?? '';
  const shouldParse =
    req.headers['content-type']?.includes('application/json') &&
    encoding !== 'zstd';

  if (shouldParse) {
    express.json({ limit: '50mb' })(req, res, next);
  } else {
    next();
  }
});

app.post('/v1/log_event', async (req, res) => {
  const { sdkType, sdkVersion } = getSdkInfo(req);
  const eventCountStr =
    req.headers?.['statsig-event-count'] ?? req.body?.events?.length;

  if (!eventCountStr) {
    console.error(
      'statsig-event-count is required',
      JSON.stringify(
        {
          path: req.path,
          params: req.params,
          headers: req.headers,
          body: req.body,
        },
        null,
        2,
      ),
    );
    throw new Error('statsig-event-count is required');
  }

  const logEventState = scrapiState?.logEvent;
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

app.all(
  ['/v2/download_config_specs/:sdk_key', '/v2/download_config_specs'],
  (_req, res) => handleDcsResponse('v2', res),
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
  const delayMs = scrapiState?.dcs?.response?.delayMs ?? 0;
  if (delayMs > 0) {
    await new Promise((r) => setTimeout(r, delayMs));
  }

  if (scrapiState?.dcs?.response?.status !== 200) {
    res.status(scrapiState?.dcs?.response?.status ?? 500).json({
      error: 'DCS is not enabled',
    });
    return;
  }

  const filepath =
    type === 'v2'
      ? scrapiState?.dcs?.response?.v2?.filepath
      : scrapiState?.dcs?.response?.v1?.filepath;
  if (filepath == null || filepath.length === 0) {
    res.status(404).json({ error: 'State not initialized' });
    return;
  }

  res.setHeader('Content-Type', 'application/json');
  res.status(200);
  const stream = createReadStream(filepath);
  stream.pipe(res);
}

function readState(): State {
  const stateContents = readFileSync('/shared-volume/state.json', 'utf8');
  return JSON.parse(stateContents);
}

function update() {
  const state = readState();
  scrapiState = state.scrapi;

  const updatedAt = new Date(state.updatedAt as unknown as string);
  if (updatedAt.getTime() != lastStateUpdateAt.getTime()) {
    console.log('State Changed');
    logStateChange(state);
  }

  lastStateUpdateAt = updatedAt;

  if (Date.now() - lastFlushedAt.getTime() < 30_000) {
    return;
  }

  const counters = takeCounters();
  if (counters.length > 0) {
    console.log(JSON.stringify(counters, null, 2));
    flushCounters(counters);
  }

  flushDockerStats();

  lastFlushedAt = new Date();
}
