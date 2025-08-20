import type { Request } from 'express';
import { readFileSync } from 'node:fs';

import { Counter } from './counters';

const benchmarkSdkKey: string = process.env.BENCH_CLUSTER_SDK_KEY ?? '';
if (!benchmarkSdkKey || benchmarkSdkKey === '') {
  throw new Error('BENCH_CLUSTER_SDK_KEY is not set');
}

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

export async function logEventsToStatsig(events: any[]) {
  const response = await fetch('https://events.statsigapi.net/v1/log_event', {
    method: 'POST',
    body: JSON.stringify({
      events,
    }),
    headers: {
      'STATSIG-API-KEY': benchmarkSdkKey,
    },
  }).catch((error) => {
    console.error('Failed to log events to Statsig', error);
    return null;
  });

  if (response == null || !response.ok) {
    console.error('Failed to log events to Statsig', response);
  } else {
    console.log(`Logged ${events.length} events to Statsig`);
  }
}

export function log(message: string, ...args: unknown[]) {
  console.log(`[${new Date().toISOString()}][scrapi] ${message}`, ...args);
}

export function flushCounters(counters: Counter[]) {
  const events = counters.map((counter) => {
    return {
      eventName: 'sdk_scenario_runner_counter',
      eventValue: counter.kind,
      eventTimestamp: Date.now(),
      metadata: counter,
    };
  });

  logEventsToStatsig(events);
}

export function flushDockerStats() {
  const statLines = readFileSync('/shared-volume/docker-stats.log', 'utf8')
    .trim()
    .split('\n');

  const stats = JSON.parse(statLines[statLines.length - 1]).stats;

  const events = stats.map((stat: any) => {
    const [received, sent] = stat.NetIO.split(' / ');
    const [read, write] = stat.BlockIO.split(' / ');
    const metadata: any = {
      name: stat.Name,
      cpuPerc: parseFloat(stat.CPUPerc.replace('%', '')),
      memBytesUsed: parseMemory(stat.MemUsage.split(' / ')[0]),
      netBytesReceived: parseMemory(received),
      netBytesSent: parseMemory(sent),
      diskBytesRead: parseMemory(read),
      diskBytesWritten: parseMemory(write),
    };

    return {
      eventName: 'sdk_scenario_runner_docker_stats',
      eventValue: stat.Name,
      eventTimestamp: Date.now(),
      metadata: metadata,
    };
  });

  console.log(JSON.stringify(events, null, 2));

  logEventsToStatsig(events);
}

function parseMemory(input: string) {
  let i = input.length - 1;
  while (input[i].match(/[a-zA-Z]/)) {
    i--;
  }

  const value = input.slice(0, i + 1);
  const unit = input.slice(i + 1);

  if (!value || !unit) {
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

    throw new Error(`Unknown unit: ${unit} for input: ${input}`);
  }

  const result = parse(value, unit);
  return Math.round(result);
}
