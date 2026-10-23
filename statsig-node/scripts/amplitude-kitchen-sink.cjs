#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const { randomUUID } = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const API_KEY = '17fcb31e58fc138462fd64bfe7add49e';
const HTTP_API = 'https://api2.amplitude.com/2/httpapi';
const TIMEOUT_MS = 30_000;
const args = process.argv.slice(2);

if (args.includes('--help') || args.includes('-h')) {
  console.log(`Usage: node scripts/amplitude-kitchen-sink.cjs [--verbose]

Runs the Rust-backed Amplitude Node API against the live Amplitude service,
using API_KEY hardcoded in this script. Events have a unique demo user and run ID.

  --verbose, -v   Print delivery callbacks, middleware, and SDK logs
  --help, -h      Show this help without sending events

Build first, from the repository root: ./tore build node --no-docker
Or run from statsig-node: npm run amplitude:kitchen-sink -- --verbose`);
} else if (args.some((arg) => !['--verbose', '-v'].includes(arg))) {
  console.error('Unknown argument. Run with --help for usage.');
  process.exitCode = 1;
} else {
  main(args.includes('--verbose') || args.includes('-v')).catch((error) => {
    console.error(`\nKitchen sink failed: ${error.message}`);
    process.exitCode = 1;
  });
}

function loadBinding() {
  // Support the source scripts/ directory and scripts copied into a built package.
  const candidates = [
    path.join(__dirname, '../build/index.js'),
    path.join(__dirname, 'index.js'),
  ];
  const entry = candidates.find((candidate) => fs.existsSync(candidate));
  if (!entry) {
    throw new Error('Node build not found. Run ./tore build node --no-docker from the repository root.');
  }
  const binding = require(entry);
  if (!binding.Amplitude || !binding.Amplitude.prototype.logEvent) {
    throw new Error('Node build is out of date. Rebuild it with ./tore build node --no-docker.');
  }
  return binding.Amplitude;
}

async function withTimeout(promise, label) {
  let timer;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`${label} timed out after ${TIMEOUT_MS / 1000}s`)), TIMEOUT_MS);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

const turn = () => new Promise((resolve) => setImmediate(resolve));
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function main(verbose) {
  const Amplitude = loadBinding();
  const runId = randomUUID();
  const userId = `kitchen-sink-${runId}`;
  const clients = new Set();
  const responses = new Map();
  const eventResponses = [];
  const shutdownEvents = new Set();
  const logs = [];
  const middlewareTrace = [];
  let steps = 0;
  let dropped = 0;
  let deliveriesWithoutCallbacks = 0;

  console.log('Amplitude Node kitchen sink — live API, Rust core');
  console.log(`Run ID: ${runId}`);
  console.log(`Demo user: ${userId}\n`);

  function event(name, properties = {}, fields = {}) {
    return {
      eventType: `Kitchen Sink: ${name}`,
      userId,
      deviceId: `node-cli-${runId}`,
      insertId: randomUUID(),
      timestamp: Date.now(),
      sessionId: -1,
      eventProperties: {
        demo: true,
        script: 'amplitude-kitchen-sink',
        run_id: runId,
        ...properties,
      },
      ...fields,
    };
  }

  const callback = (event, status, message) => {
    responses.set(event.insertId, { event, status, message });
    if (verbose || (!(status >= 200 && status < 300) && !shutdownEvents.has(event.insertId))) {
      console.log(`  callback ${status}: ${event.eventType} — ${message}`);
    }
  };
  const perEventCallback = (event, status, message) => {
    eventResponses.push({ event, status, message });
    if (verbose) console.log(`  event callback ${status}: ${event.eventType}`);
  };
  const logger = Object.fromEntries(['debug', 'warn', 'error'].map((level) => [level, (tag, message) => {
    logs.push({ level, tag, message });
    if (verbose || level === 'error') console.log(`  ${level} [${tag}]: ${message}`);
  }]));

  async function step(label, action) {
    await withTimeout(action(), label);
    steps += 1;
    console.log(`✓ ${label}`);
  }

  async function delivered(...events) {
    await withTimeout((async () => {
      while (events.some((event) => !responses.has(event.insertId))) await sleep(5);
    })(), 'Delivery callbacks');
    for (const event of events) {
      const result = responses.get(event.insertId);
      assert.ok(result.status >= 200 && result.status < 300,
        `${event.eventType}: HTTP ${result.status}, ${result.message}`);
    }
  }

  const amplitude = Amplitude.getInstance('kitchen-sink');
  clients.add(amplitude);

  try {
    await step('getInstance: default, named, and shared identities', async () => {
      const defaultClient = Amplitude.getInstance();
      clients.add(defaultClient);
      assert.equal(defaultClient, Amplitude.getInstance(''));
      assert.equal(defaultClient, Amplitude.getInstance(null));
      assert.equal(amplitude, Amplitude.getInstance('kitchen-sink'));
      assert.notEqual(defaultClient, amplitude);
    });

    await step('init, logger, options, plans, timeouts, pools, and throttling configuration', async () => {
      amplitude.init(API_KEY);
      amplitude.setLogger(logger).setLogMode(verbose ? 'DEBUG' : 'ERROR');
      assert.equal(amplitude.setEventUploadThreshold(3), amplitude);
      amplitude.setEventUploadPeriodMillis(10_000)
        .setFlushTimeout(15_000)
        .setSendThreadPool(2)
        .setRetryThreadPool(2)
        .setRecordThrottledId(true)
        .setProxy(null)
        .setOptions({ minIdLength: 5, headers: { 'X-Amplitude-Kitchen-Sink': 'rust-node' } })
        .setPlan({ branch: 'demo', source: 'node-cli', version: '1', versionId: runId })
        .setIngestionMetadata({ sourceName: 'amplitude-kitchen-sink', sourceVersion: '1' })
        .setCallbacks({ onLogEventServerResponse: callback });
      assert.equal(amplitude.shouldWait(event('Backpressure probe')), false);
    });

    await step('addEventMiddleware: continuation, extra data, and dropping an event', async () => {
      amplitude.addEventMiddleware(async (payload, next) => {
        middlewareTrace.push(`before:${payload.event.eventType}`);
        payload.event.eventProperties = {
          ...payload.event.eventProperties,
          middleware: 'Rust-backed Node continuation',
          ...(payload.extra == null ? {} : { middleware_extra: payload.extra }),
        };
        await next(payload);
        middlewareTrace.push(`after:${payload.event.eventType}`);
      });
      amplitude.addEventMiddleware(async (payload, next) => {
        if (payload.event.eventProperties.skip_kitchen_sink) {
          dropped += 1;
          return;
        }
        await next(payload);
      });
      await amplitude.logEvent(event('Dropped by middleware', { skip_kitchen_sink: true }));
      assert.equal(dropped, 1);
      assert.equal(responses.size, 0);
    });

    await step('track: immediate delivery, device fields, groups, revenue, and metadata overrides', async () => {
      const rich = event('Rich event', { nested: { tags: ['demo', 'node'], enabled: true, score: 1.5 } }, {
        appVersion: 'kitchen-sink-1',
        platform: 'Node.js', osName: process.platform, osVersion: process.version,
        deviceBrand: 'Demo', deviceManufacturer: 'Amplitude kitchen sink', deviceModel: 'Node CLI',
        locationLat: 0, locationLng: 0, language: 'en', partnerId: 'kitchen-sink',
        userProperties: { demo_user: true, run_id: runId },
        groups: { demo_account: 'kitchen-sink' }, groupProperties: { demo_account: { demo: true } },
        price: 1.25, quantity: 2, revenueType: 'demo', productId: 'kitchen-sink-product', currency: 'USD',
        eventId: 1,
        plan: { source: 'event-override', version: '1' },
        ingestionMetadata: { sourceName: 'kitchen-sink-rich-event', sourceVersion: '1' },
      });
      await amplitude.track(rich);
      await delivered(rich);
      assert.equal(responses.get(rich.insertId).event.plan.source, 'event-override');
    });

    await step('logEvent: threshold batching, all overloads, and client/event callbacks', async () => {
      const one = event('Buffered function callback');
      const two = event('Buffered object callback');
      const three = event('Buffered extra only');
      await amplitude.logEvent(one, perEventCallback, { step: 'buffer', ordinal: 1 });
      await amplitude.logEvent(two, { onLogEventServerResponse: perEventCallback }, { step: 'buffer', ordinal: 2 });
      await amplitude.logEvent(three, { step: 'buffer', ordinal: 3 });
      await amplitude.flushEvents();
      await delivered(one, two, three);
      await turn();
      assert.equal(eventResponses.length, 2);
      assert.ok(eventResponses.every(({ status }) => status >= 200 && status < 300));
      assert.equal(responses.get(one.insertId).event.eventProperties.middleware_extra.ordinal, 1);
      assert.equal(responses.get(one.insertId).event.plan.source, 'node-cli');
    });

    await step('useBatchMode: deliver through the Batch API, then restore regular mode', async () => {
      amplitude.useBatchMode(true);
      const batch = event('Batch API');
      await amplitude.track(batch);
      await delivered(batch);
      amplitude.useBatchMode(false);
    });

    await step('setEventUploadPeriodMillis: automatic background flush', async () => {
      amplitude.setEventUploadThreshold(10).setEventUploadPeriodMillis(50);
      const periodic = event('Periodic flush');
      await amplitude.logEvent(periodic);
      await delivered(periodic);
      amplitude.setEventUploadPeriodMillis(10_000);
    });

    await step('setServerUrl and flushEvents: custom endpoint and null callback overload', async () => {
      amplitude.setServerUrl(HTTP_API);
      const manual = event('Explicit flush');
      await amplitude.logEvent(manual, null, { step: 'explicit-flush' });
      await amplitude.flushEvents();
      await delivered(manual);
    });

    await step('clear optional settings and callbacks; retain immediate track compatibility', async () => {
      amplitude.setCallbacks(null).setPlan(null).setIngestionMetadata(null).setOptions(null);
      const before = responses.size;
      await amplitude.track(event('Cleared optional settings'));
      deliveriesWithoutCallbacks += 1;
      await turn();
      assert.equal(responses.size, before);
      amplitude.setCallbacks(callback);
    });

    await step('new Amplitude: independent client with init options', async () => {
      const independent = new Amplitude();
      clients.add(independent);
      independent.init(API_KEY, { serverUrl: HTTP_API, timeoutMs: 10_000 });
      independent.setLogger(logger).setCallbacks(callback);
      const ownEvent = event('Independent client');
      await independent.track(ownEvent);
      await delivered(ownEvent);
      await independent.shutdown();
    });

    await step('local validation: reject a missing identity without sending it', async () => {
      await assert.rejects(amplitude.track({ eventType: 'Kitchen Sink: Invalid local event' }), /user_id or device_id/);
    });

    await step('shutdown: buffered-event callback, rejection after close, and fresh named instance', async () => {
      amplitude.setEventUploadThreshold(100).setEventUploadPeriodMillis(10_000);
      const pending = event('Buffered shutdown callback — not uploaded');
      shutdownEvents.add(pending.insertId);
      await amplitude.logEvent(pending);
      await amplitude.shutdown();
      await withTimeout((async () => {
        while (!responses.has(pending.insertId)) await sleep(5);
      })(), 'Shutdown callback');
      assert.equal(responses.get(pending.insertId).status, 0);
      await assert.rejects(amplitude.logEvent(event('After shutdown — not uploaded')), /shut down/);
      const replacement = Amplitude.getInstance('kitchen-sink');
      clients.add(replacement);
      assert.notEqual(replacement, amplitude);
      await replacement.shutdown();
    });

    const failures = [...responses.values()].filter(({ event, status }) =>
      !shutdownEvents.has(event.insertId) && !(status >= 200 && status < 300));
    assert.equal(failures.length, 0, 'Unexpected delivery failures');
    const successful = [...responses.values()].filter(({ status }) => status >= 200 && status < 300).length;
    console.log(`\nPassed ${steps} scenarios. ${successful + deliveriesWithoutCallbacks} demo events accepted by Amplitude.`);
    console.log(`Also checked: ${dropped} middleware drop, ${eventResponses.length} event callbacks, and a status-0 shutdown callback.`);
    if (verbose) console.log(`${logs.length} SDK logs; ${middlewareTrace.length} middleware trace entries.`);
  } finally {
    await withTimeout(Promise.all([...clients].map((client) => client.shutdown())), 'Client cleanup');
    await turn();
  }
}
