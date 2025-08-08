import express from 'express';
import _ from 'lodash';
import fs from 'node:fs';

const evalProjSdkKey: string = process.env.EVAL_PROJ_SDK_KEY ?? '';
if (!evalProjSdkKey || evalProjSdkKey === '') {
  throw new Error('EVAL_PROJ_SDK_KEY is not set');
}

const benchmarkSdkKey: string = process.env.BENCH_CLUSTER_SDK_KEY ?? '';
if (!benchmarkSdkKey || benchmarkSdkKey === '') {
  throw new Error('BENCH_CLUSTER_SDK_KEY is not set');
}

const cdnUrl = 'https://api.statsigcdn.com';
const counters = {};

const dcsV2 = await fetch(
  `${cdnUrl}/v2/download_config_specs/${evalProjSdkKey}.json`,
).then((res) => res.json());

const dcsV1 = await fetch(
  `${cdnUrl}/v1/download_config_specs/${evalProjSdkKey}.json`,
).then((res) => res.json());

delete dcsV1.hashed_sdk_key_used;

const idListFiles = {};
const idListsV1 = await fetch(
  `${cdnUrl}/v1/get_id_lists/${evalProjSdkKey}.json`,
)
  .then((res) => res.json())
  .then((id_lists) => {
    const mapped = {};
    for (const [key, value] of Object.entries(id_lists)) {
      const data = value as any;

      fetch(data.url)
        .then((res) => res.text())
        .then((text) => {
          idListFiles[key] = text;
        });

      mapped[key] = {
        ...data,
        url: `http://scrapi:8000/v1/download_id_list_file/${key}`,
      };
    }

    return mapped;
  });

const app = express();

app.use((req, res, next) => {
  const shouldParse =
    req.headers['content-type'] === 'application/json' &&
    req.headers['content-encoding'] !== 'zstd';

  if (shouldParse) {
    express.json({ limit: '10mb' })(req, res, next);
  } else {
    next();
  }
});

app.use((req, _res, next) => {
  const { sdkType, sdkVersion } = getSdkInfo(req);

  if (sdkType === 'unknown') {
    console.log(`${req.method} ${req.path}`);
    return next();
  }

  const key = `req_count_${req.method}_${req.path}_${sdkType}@${sdkVersion}`;
  const entry = counters[key] ?? {
    sdkType,
    sdkVersion,
    path: req.path,
    count: 0,
  };
  entry.count += 1;
  counters[key] = entry;

  console.log(`${req.method} ${req.path} from ${sdkType}@${sdkVersion}`);
  next();
});

app.post('/v1/log_event', (req, res) => {
  const { sdkType, sdkVersion } = getSdkInfo(req);
  const eventCount =
    req.headers?.['statsig-event-count'] ?? req.body?.events?.length;

  if (!eventCount) {
    throw new Error('statsig-event-count is required');
  }

  const eventCountInt = parseInt(eventCount as string);
  const eventCountKey = `event_count_${sdkType}@${sdkVersion}`;
  const eventCountEntry = counters[eventCountKey] ?? {
    sdkType,
    sdkVersion,
    counts: [],
  };
  eventCountEntry.counts.push(eventCountInt);
  counters[eventCountKey] = eventCountEntry;

  res.status(202).json({ success: true });
});

app.all(
  ['/v1/download_config_specs/:sdk_key', '/v1/download_config_specs'],
  async (_req, res) => {
    res.status(200).json(dcsV1);
  },
);

app.all('/v2/download_config_specs/:sdk_key', async (_req, res) => {
  res.status(200).json(dcsV2);
});

app.all('/v1/get_id_lists', (_req, res) => {
  res.status(200).json(idListsV1);
});

app.all('/v1/download_id_list_file/:id_list_name', async (req, res) => {
  const idListName = req.params.id_list_name;
  if (idListFiles[idListName]) {
    res.status(200).send(idListFiles[idListName]);
  } else {
    res.status(404).json({ error: 'ID list not found' });
  }
});

app.all('/alive', (_req, res) => {
  res.status(200).send();
});

app.all('/shutdown', (_req, res) => {
  res.status(200).send();

  postResults().then(() => {
    process.exit(0);
  });
});

app.listen(8000, () => {
  console.log('Server is running on port 8000');
  writeSpecNamesToFile(dcsV2);
});

function writeSpecNamesToFile(dcs: any) {
  const names: any = {
    feature_gates: [],
    dynamic_configs: [],
    experiments: [],
    layers: [],
  };

  Object.entries(dcs.feature_gates).forEach(([name, spec]: [string, any]) => {
    if (spec.entity === 'feature_gate') {
      names.feature_gates.push(name);
    }
  });

  Object.entries(dcs.dynamic_configs).forEach(([name, spec]: [string, any]) => {
    if (spec.entity === 'dynamic_config') {
      names.dynamic_configs.push(name);
    } else if (spec.entity === 'experiment' || spec.entity === 'autotune') {
      names.experiments.push(name);
    }
  });

  Object.entries(dcs.layer_configs).forEach(([name, spec]: [string, any]) => {
    if (spec.entity === 'layer') {
      names.layers.push(name);
    }
  });

  fs.writeFileSync(
    '/shared-volume/spec_names.json',
    JSON.stringify(names, null, 2),
  );
}

type RawStats = {
  BlockIO: string;
  CPUPerc: string;
  Container: string;
  ID: string;
  MemPerc: string;
  MemUsage: string;
  Name: string;
  NetIO: string;
  PIDs: string;
};

type ProcessStats = {
  cpuPerc: number;
  memBytesUsed: number;
  netBytesReceived: number;
  netBytesSent: number;
  diskBytesRead: number;
  diskBytesWritten: number;
  name: string;
};

type StatsLine = {
  timestamp: number;
  stats: RawStats[];
};

type BenchmarkResult = {
  p99: number;
  max: number;
  min: number;
  median: number;
  avg: number;
  benchmarkName: string;
  specName: string;
  sdkType: string;
  sdkVersion: string;
};

type ResultData = {
  sdkType: string;
  sdkVersion: string;
  results: BenchmarkResult[];
};

async function postResults() {
  const { events, sdkVersionMapping } = processBenchmarks();
  const dockerEvents = processDockerStats(sdkVersionMapping);

  const counterEvents: any[] = [];
  for (const [key, value] of Object.entries(counters)) {
    let metadata: any = {};
    const data = value as any;
    let logValue = key;

    if (key.startsWith('req_count_')) {
      logValue = data.path;
      metadata = {
        type: 'req_count',
        sdkType: data.sdkType,
        sdkVersion: data.sdkVersion,
        numRequests: data.count,
        path: data.path,
      };
    } else if (key.startsWith('event_count_')) {
      const sorted = data.counts.sort((a: number, b: number) => a - b);
      const sum = sorted.reduce((a: number, b: number) => a + b, 0);
      metadata = {
        type: 'event_count',
        sdkType: data.sdkType,
        sdkVersion: data.sdkVersion,
        sum,
        p99: sorted[Math.floor(sorted.length * 0.99)],
        max: sorted[sorted.length - 1],
        min: sorted[0],
        median: sorted[Math.floor(sorted.length / 2)],
        avg: sorted.reduce((a: number, b: number) => a + b, 0) / sorted.length,
      };
      console.assert(metadata.min <= metadata.max, 'min <= max');
    } else {
      throw new Error(`Unknown counter key: ${key}`);
    }

    counterEvents.push({
      eventName: 'sdk_bench_cluster_counter',
      value: logValue,
      user: { userID: 'bench_cluster' },
      time: Date.now(),
      metadata,
    });
  }

  const allEvents = [...events, ...dockerEvents, ...counterEvents];

  const debugEventCounts = allEvents.reduce((acc, event) => {
    const key = `${event.eventName}_${event.metadata.sdkType}@${event.metadata.sdkVersion}`;
    const entry = acc[key] ?? {
      eventName: event.eventName,
      sdkType: event.metadata.sdkType,
      sdkVersion: event.metadata.sdkVersion,
      count: 0,
    };
    entry.count += 1;
    acc[key] = entry;
    return acc;
  }, {});

  console.log(JSON.stringify(debugEventCounts, null, 2));

  const chunks = _.chunk(allEvents, 900);
  await Promise.all(
    chunks.map(async (chunk) => {
      console.log(`Posting ${chunk.length} events`);
      await fetch('https://events.statsigapi.net/v1/log_event', {
        method: 'POST',
        body: JSON.stringify({
          events: chunk,
        }),
        headers: {
          'STATSIG-API-KEY': benchmarkSdkKey,
        },
      });
    }),
  );
}

function processBenchmarks() {
  const list = fs.readdirSync('/shared-volume');
  const events: any[] = [];
  const sdkVersionMapping = {};

  for (const file of list) {
    if (!file.endsWith('-results.json')) {
      continue;
    }

    const contents = fs.readFileSync(`/shared-volume/${file}`, 'utf8');
    const data: ResultData = JSON.parse(contents);

    sdkVersionMapping[data.sdkType] = data.sdkVersion;

    for (const result of data.results) {
      const metadata = {
        sdkType: data.sdkType,
        sdkVersion: data.sdkVersion,
        benchmarkName: result.benchmarkName,
        specName: result.specName,
        p99: result.p99,
        max: result.max,
        min: result.min,
        median: result.median,
        avg: result.avg,
      };

      validateBenchmarkMetadata(metadata);

      events.push({
        eventName: 'sdk_bench_cluster_benchmark',
        value: result.benchmarkName,
        user: { userID: 'bench_cluster' },
        time: Date.now(),
        metadata,
      });
    }
  }

  return {
    events,
    sdkVersionMapping,
  };
}

function validateBenchmarkMetadata(metadata: Record<string, unknown>) {
  const {
    sdkType,
    sdkVersion,
    benchmarkName,
    specName,
    p99,
    max,
    min,
    median,
    avg,
  } = metadata;

  if (sdkType == null || sdkType === 'statsig-server-core') {
    throw new Error(`Invalid SDK type: ${sdkType}`);
  }

  if (sdkVersion == null || sdkVersion === 'unknown') {
    throw new Error(`Invalid SDK version: ${sdkVersion}`);
  }

  if (benchmarkName == null || benchmarkName === 'unknown') {
    throw new Error(`Invalid benchmark name: ${benchmarkName}`);
  }

  if (specName == null) {
    throw new Error(`Invalid spec name: ${specName}`);
  }

  if (p99 == null || isNaN(parseFloat(p99 as string))) {
    throw new Error(`Invalid p99: ${p99}`);
  }

  if (max == null || isNaN(parseFloat(max as string))) {
    throw new Error(`Invalid max: ${max}`);
  }

  if (min == null || isNaN(parseFloat(min as string))) {
    throw new Error(`Invalid min: ${min}`);
  }

  if (median == null || isNaN(parseFloat(median as string))) {
    throw new Error(`Invalid median: ${median}`);
  }

  if (avg == null || isNaN(parseFloat(avg as string))) {
    throw new Error(`Invalid avg: ${avg}`);
  }
}

function processDockerStats(sdkVersionMapping: Record<string, string>) {
  const stats = fs.readFileSync('/shared-volume/docker-stats.log', 'utf8');
  const statLines = stats.split('\n').filter((line) => line.trim() !== '');

  const rawStats: RawStats[] = [];
  for (const line of statLines) {
    const stat: StatsLine = JSON.parse(line);
    rawStats.push(...stat.stats);
  }

  const processStats: Record<string, ProcessStats[]> = {};
  for (const stat of rawStats) {
    const [received, sent] = stat.NetIO.split(' / ');
    const [read, write] = stat.BlockIO.split(' / ');
    const processed: ProcessStats = {
      name: stat.Name,
      cpuPerc: parseFloat(stat.CPUPerc.replace('%', '')),
      memBytesUsed: parseMemory(stat.MemUsage.split(' / ')[0]),
      netBytesReceived: parseMemory(received),
      netBytesSent: parseMemory(sent),
      diskBytesRead: parseMemory(read),
      diskBytesWritten: parseMemory(write),
    };

    const arr = processStats[stat.Name] ?? [];
    arr.push(processed);
    processStats[stat.Name] = arr;
  }

  const entries = Object.entries(processStats);
  const events: any[] = [];
  for (const [name, stats] of entries) {
    const cpuStats = getStatsForField(stats, 'cpuPerc');
    const memStats = getStatsForField(stats, 'memBytesUsed');
    const netReceivedStats = getStatsForField(stats, 'netBytesReceived');
    const netSentStats = getStatsForField(stats, 'netBytesSent');
    const diskReadStats = getStatsForField(stats, 'diskBytesRead');
    const diskWriteStats = getStatsForField(stats, 'diskBytesWritten');

    const metadata: any = {
      cpuPercP99: cpuStats.p99,
      cpuPercMax: cpuStats.max,
      cpuPercMin: cpuStats.min,
      cpuPercMedian: cpuStats.median,
      cpuPercAvg: cpuStats.avg,

      memBytesUsedP99: memStats.p99,
      memBytesUsedMax: memStats.max,
      memBytesUsedMin: memStats.min,
      memBytesUsedMedian: memStats.median,
      memBytesUsedAvg: memStats.avg,

      netBytesSentP99: netSentStats.p99,
      netBytesSentMax: netSentStats.max,
      netBytesSentMin: netSentStats.min,
      netBytesSentMedian: netSentStats.median,
      netBytesSentAvg: netSentStats.avg,

      netBytesReceivedP99: netReceivedStats.p99,
      netBytesReceivedMax: netReceivedStats.max,
      netBytesReceivedMin: netReceivedStats.min,
      netBytesReceivedMedian: netReceivedStats.median,
      netBytesReceivedAvg: netReceivedStats.avg,

      diskBytesWrittenP99: diskWriteStats.p99,
      diskBytesWrittenMax: diskWriteStats.max,
      diskBytesWrittenMin: diskWriteStats.min,
      diskBytesWrittenMedian: diskWriteStats.median,
      diskBytesWrittenAvg: diskWriteStats.avg,

      diskBytesReadP99: diskReadStats.p99,
      diskBytesReadMax: diskReadStats.max,
      diskBytesReadMin: diskReadStats.min,
      diskBytesReadMedian: diskReadStats.median,
      diskBytesReadAvg: diskReadStats.avg,
    };

    const sdkType = getSdkTypeForService(name);
    if (sdkType) {
      const sdkVersion = sdkVersionMapping[sdkType] ?? 'unknown';
      metadata.sdkType = sdkType;
      metadata.sdkVersion = sdkVersion;
    }

    events.push({
      eventName: 'sdk_bench_cluster_docker_stats',
      value: name,
      user: { userID: 'bench_cluster' },
      time: Date.now(),
      metadata: {
        ...metadata,
      },
    });
  }

  return events;
}

function getStatsForField(stats: ProcessStats[], field: keyof ProcessStats) {
  const sorted = stats.sort((a: any, b: any) => a[field] - b[field]);
  const p99 = sorted[Math.floor(stats.length * 0.99)];
  const max = sorted[stats.length - 1];
  const min = sorted[0];
  const median = sorted[Math.floor(stats.length / 2)];
  const avg = stats.reduce((a: any, b: any) => a + b[field], 0) / stats.length;
  console.assert(min <= max, 'min <= max');

  return {
    p99: p99[field],
    max: max[field],
    min: min[field],
    median: median[field],
    avg,
  };
}

function getSdkTypeForService(name: string) {
  switch (name) {
    case 'scrapi':
    case 'docker-stats':
      return null;

    // SDKs
    case 'dotnet-core':
      return 'statsig-server-core-dotnet';
    case 'dotnet-legacy':
      return 'dotnet-server';

    case 'go-core':
      return 'statsig-server-core-go';
    case 'go-legacy':
      return 'go-sdk';

    case 'java-core':
      return 'statsig-server-core-java';
    case 'java-legacy':
      return 'java-server';

    case 'node-core':
      return 'statsig-server-core-node';
    case 'node-legacy':
      return 'statsig-node';

    case 'php-core':
      return 'statsig-server-core-php';
    case 'php-legacy':
      return 'php-server';

    case 'python-core':
      return 'statsig-server-core-python';
    case 'python-legacy':
      return 'py-server';

    // case 'ruby-core':
    //   return 'statsig-server-core-ruby';
    case 'ruby-legacy':
      return 'ruby-server';

    case 'rust-core':
      return 'statsig-server-core-rust';
    case 'rust-legacy':
      return 'rust-server';
  }

  throw new Error(`Unknown service: ${name}`);
}

function parseMemory(input: string) {
  const parts = input.match(/^([0-9.]+)([a-zA-Z]+)$/);
  if (!parts) {
    throw new Error(`Unknown memory format: ${input}`);
  }

  function parse(value: string, unit: string) {
    const lowerUnit = unit.toLowerCase();
    if (lowerUnit === 'b') {
      return parseFloat(value);
    }

    if (lowerUnit === 'kb') {
      return parseFloat(value) * 1000;
    }

    if (lowerUnit === 'kib') {
      return parseFloat(value) * 1024;
    }

    if (lowerUnit === 'mb') {
      return parseFloat(value) * 1000 * 1000;
    }

    if (lowerUnit === 'mib') {
      return parseFloat(value) * 1024 * 1024;
    }

    if (lowerUnit === 'gb') {
      return parseFloat(value) * 1000 * 1000 * 1000;
    }

    if (lowerUnit === 'gib') {
      return parseFloat(value) * 1024 * 1024 * 1024;
    }

    throw new Error(`Unknown unit: ${unit}`);
  }

  const [, value, unit] = parts;
  const result = parse(value, unit);
  return Math.round(result);
}

function getSdkInfo(req: any) {
  const sdkType =
    req.headers?.['statsig-sdk-type'] ??
    req.body?.statsigMetadata?.sdkType ??
    'unknown';
  const sdkVersion =
    req.headers?.['statsig-sdk-version'] ??
    req.body?.statsigMetadata?.sdkVersion ??
    'unknown';

  return {
    sdkType,
    sdkVersion,
  };
}
