import express from 'express';
import { execSync } from 'node:child_process';
import {
  createWriteStream,
  existsSync,
  readFileSync,
  renameSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { State } from '../common';
import { INITIAL_STATE } from './initial_state';

await boot();

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const app = express();

app.use(express.json({ limit: '10mb' }));
app.use(express.static(join(__dirname, 'public')));

app.use((req, _res, next) => {
  log(`${req.method} ${req.path}`);
  next();
});

app.get('/', async (_req, res) => {
  res.sendFile(__dirname + '/public/index.html');
});

app.post('/state', async (req, res) => {
  log('Received state', req.body);
  if (req.body && Object.keys(req.body).length > 0) {
    log('Writing state', req.body);
    req.body.updatedAt = new Date();
    writeState(req.body);
  }

  const state = readState();
  log('Sending state', state);
  res.status(200).json(state);
});

app.get('/stats', async (_req, res) => {
  const dockerStats = readDockerStats();
  const perfStats = readPerfStats();
  res.status(200).json({ dockerStats, perfStats });
});

app.listen(80, async () => {
  log('Dashboard server running');

  const state = readState();
  if (state.scrapi.dcs.syncing.enabled) {
    await syncDcs(state);
  }

  setInterval(update, 1000);
});

async function boot() {
  if (INITIAL_STATE.scrapi.dcs.syncing.enabled) {
    await syncDcs(INITIAL_STATE);
  }

  for (let i = 0; i < 10; i++) {
    if (existsSync('/shared-volume/docker-stats.log')) {
      break;
    }

    console.log('Waiting for docker-stats.log to be ready');
    await new Promise((r) => setTimeout(r, 1000));
  }
}

function log(message: string, ...args: unknown[]) {
  console.log(`[${new Date().toISOString()}][dashboard] ${message}`, ...args);
}

function writeState(state: Record<string, unknown>) {
  writeFileSync(
    '/shared-volume/temp_state.json',
    JSON.stringify(state, null, 2),
  );

  execSync('mv /shared-volume/temp_state.json /shared-volume/state.json');
}

function readState() {
  const contents = readFileSync('/shared-volume/state.json', 'utf8');
  return JSON.parse(contents);
}

function readDockerStats() {
  const contents = readFileSync('/shared-volume/docker-stats.log', 'utf8');
  const lines = contents.split('\n');
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i];
    if (line.trim() !== '') {
      return JSON.parse(line);
    }
  }
  return {};
}

function readPerfStats() {
  const services = ['node', 'python', 'java', 'go', 'dotnet'];
  const results = {};

  for (const service of services) {
    for (const variant of ['core', 'legacy']) {
      try {
        const contents = readFileSync(
          `/shared-volume/profile-${service}-${variant}.json`,
          'utf8',
        );
        results[`${service}-${variant}`] = JSON.parse(contents);
      } catch (error) {
        continue;
      }
    }
  }

  return results;
}

let isUpdating = false;
async function update() {
  if (isUpdating) {
    return;
  }

  isUpdating = true;

  const state = readState();
  const dcsState = state.scrapi.dcs;
  const now = new Date();

  if (dcsState.syncing.enabled) {
    const updatedAt = new Date(dcsState.syncing.updatedAt);
    if (now.getTime() - updatedAt.getTime() > dcsState.syncing.intervalMs) {
      await syncDcs(state);
    }
  }

  isUpdating = false;
}

async function syncDcs(state: State) {
  log('Syncing DCS');
  const sdkKey = state.scrapi.dcs.syncing.sdkKey;
  try {
    const [v2Filesize, v1Filesize] = await Promise.all([
      fetchAndWrite(
        `https://api.statsigcdn.com/v2/download_config_specs/${sdkKey}.json`,
        '/shared-volume/dcs-v2.json',
      ),
      fetchAndWrite(
        `https://api.statsigcdn.com/v1/download_config_specs/${sdkKey}.json`,
        '/shared-volume/dcs-v1.json',
      ),
    ]);

    if (v2Filesize == -1 || v1Filesize == -1) {
      throw new Error('Failed to fetch DCS');
    }

    state.scrapi.dcs.response.v2.filesize = v2Filesize;
    state.scrapi.dcs.response.v1.filesize = v1Filesize;
    state.scrapi.dcs.response.v2.filepath = '/shared-volume/dcs-v2.json';
    state.scrapi.dcs.response.v1.filepath = '/shared-volume/dcs-v1.json';
    state.scrapi.dcs.syncing.updatedAt = new Date();

    writeState(state);
    log('Successfully synced DCS');
  } catch (error) {
    log('Error polling DCS', error);
    return;
  }
}

async function fetchAndWrite(url: string, filepath: string): Promise<number> {
  log('Fetching and writing', url, filepath);
  try {
    const res = await fetch(url);
    if (!res.ok || !res.body) {
      throw new Error(`Failed to fetch ${url}: ${res.statusText}`);
    }

    const tmpPath = `${filepath}.tmp`;
    const reader = res.body.getReader();
    const writer = createWriteStream(tmpPath);

    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      writer.write(value);
    }

    writer.end();
    writer.close();
    renameSync(tmpPath, filepath);

    const stats = statSync(filepath);
    return stats.size;
  } catch (error) {
    log('Error fetching and writing', url, error);
    return -1;
  }
}
